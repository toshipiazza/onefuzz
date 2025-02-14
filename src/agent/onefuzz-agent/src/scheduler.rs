// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use std::fmt;

use anyhow::Result;
use onefuzz::process::Output;

use crate::commands::add_ssh_key;
use crate::coordinator::{NodeCommand, NodeState};
use crate::reboot::RebootContext;
use crate::setup::ISetupRunner;
use crate::work::*;
use crate::worker::*;

#[derive(Debug)]
pub enum Scheduler {
    Free(State<Free>),
    SettingUp(State<SettingUp>),
    PendingReboot(State<PendingReboot>),
    Ready(State<Ready>),
    Busy(State<Busy>),
    Done(State<Done>),
}

impl From<&Scheduler> for NodeState {
    fn from(value: &Scheduler) -> Self {
        match value {
            Scheduler::Free(_) => Self::Free,
            Scheduler::SettingUp(_) => Self::SettingUp,
            Scheduler::PendingReboot(_) => Self::Rebooting,
            Scheduler::Ready(_) => Self::Ready,
            Scheduler::Busy(_) => Self::Busy,
            Scheduler::Done(_) => Self::Done,
        }
    }
}

impl fmt::Display for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Free(..) => "Scheduler::Free",
            Self::SettingUp(..) => "Scheduler::SettingUp",
            Self::PendingReboot(..) => "Scheduler::PendingReboot",
            Self::Ready(..) => "Scheduler::Ready",
            Self::Busy(..) => "Scheduler::Busy",
            Self::Done(..) => "Scheduler::Done",
        };
        write!(f, "{s}")
    }
}

impl Scheduler {
    pub fn new(ctx: Option<RebootContext>) -> Self {
        if let Some(ctx) = ctx {
            let work_set = ctx.work_set;
            let ctx = Ready { work_set };
            let state = State { ctx };
            state.into()
        } else {
            let state = State { ctx: Free {} };
            state.into()
        }
    }

    pub async fn execute_command(self, cmd: NodeCommand, managed: bool) -> Result<Self> {
        match cmd {
            NodeCommand::AddSshKey(ssh_key_info) => {
                if managed {
                    add_ssh_key(&ssh_key_info).await?;
                } else {
                    warn!("adding ssh keys only supported on managed nodes");
                }
                Ok(self)
            }
            NodeCommand::StopTask(stop_task) => {
                if let Scheduler::Busy(state) = self {
                    let state = state.stop(stop_task.task_id).await?;
                    Ok(state.into())
                } else {
                    Ok(self)
                }
            }
            NodeCommand::Stop {} => {
                let cause = DoneCause::Stopped;
                let state = State {
                    ctx: Done { cause },
                };
                Ok(state.into())
            }
            NodeCommand::StopIfFree {} => {
                if let Scheduler::Free(_) = self {
                    let cause = DoneCause::Stopped;
                    let state = State {
                        ctx: Done { cause },
                    };
                    Ok(state.into())
                } else {
                    Ok(self)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Free;

#[derive(Debug)]
pub struct SettingUp {
    work_set: WorkSet,
}

#[derive(Debug)]
pub struct PendingReboot {
    work_set: WorkSet,
}

#[derive(Debug)]
pub struct Ready {
    work_set: WorkSet,
}

#[derive(Debug)]
pub struct Busy {
    workers: Vec<Option<Worker>>,
}

#[derive(Debug)]
pub struct Done {
    cause: DoneCause,
}

#[derive(Clone, Debug)]
pub enum DoneCause {
    SetupError {
        error: String,
        script_output: Option<Output>,
    },
    Stopped,
    WorkersDone,
}

pub trait Context {}

impl Context for Free {}
impl Context for SettingUp {}
impl Context for PendingReboot {}
impl Context for Ready {}
impl Context for Busy {}
impl Context for Done {}

#[derive(Debug)]
pub struct State<C: Context> {
    ctx: C,
}

macro_rules! impl_from_state_for_scheduler {
    ($Context: ident) => {
        impl From<State<$Context>> for Scheduler {
            fn from(state: State<$Context>) -> Self {
                Scheduler::$Context(state)
            }
        }
    };
}

impl_from_state_for_scheduler!(Free);
impl_from_state_for_scheduler!(SettingUp);
impl_from_state_for_scheduler!(PendingReboot);
impl_from_state_for_scheduler!(Ready);
impl_from_state_for_scheduler!(Busy);
impl_from_state_for_scheduler!(Done);

impl<C: Context> From<C> for State<C> {
    fn from(ctx: C) -> Self {
        State { ctx }
    }
}

impl State<Free> {
    pub fn schedule(self, work_set: WorkSet) -> State<SettingUp> {
        let ctx = SettingUp { work_set };
        State { ctx }
    }
}

pub enum SetupDone {
    Ready(State<Ready>),
    PendingReboot(State<PendingReboot>),
    Done(State<Done>),
}

impl State<SettingUp> {
    pub async fn finish(self, runner: &dyn ISetupRunner) -> Result<SetupDone> {
        let work_set = self.ctx.work_set;

        let output = runner.run(&work_set).await;

        match output {
            Ok(Some(output)) => {
                if !output.exit_status.success {
                    let error = "error running target setup script".to_owned();
                    warn!("{}", error);
                    let cause = DoneCause::SetupError {
                        error,
                        script_output: Some(output),
                    };
                    let ctx = Done { cause };
                    return Ok(SetupDone::Done(ctx.into()));
                }
            }
            Ok(None) => {
                // No script was executed.
            }
            Err(err) => {
                let error = format!("{err:?}");
                warn!("{}", error);
                let cause = DoneCause::SetupError {
                    error,
                    script_output: None,
                };
                let ctx = Done { cause };
                return Ok(SetupDone::Done(ctx.into()));
            }
        }

        let done = if work_set.reboot {
            let ctx = PendingReboot { work_set };
            SetupDone::PendingReboot(ctx.into())
        } else {
            let ctx = Ready { work_set };
            SetupDone::Ready(ctx.into())
        };

        Ok(done)
    }

    pub fn work_set(&self) -> &WorkSet {
        &self.ctx.work_set
    }
}

impl State<PendingReboot> {
    pub fn reboot_context(self) -> RebootContext {
        RebootContext::new(self.ctx.work_set)
    }
}

impl State<Ready> {
    pub async fn run(self, machine_id: uuid::Uuid) -> Result<State<Busy>> {
        let mut workers = vec![];
        let setup_dir = self.ctx.work_set.setup_dir()?;
        let extra_setup_dir = self.ctx.work_set.extra_setup_dir()?;

        for work in self.ctx.work_set.work_units {
            let work_dir = work.working_dir(machine_id)?;
            let worker = Some(Worker::new(
                work_dir,
                setup_dir.clone(),
                extra_setup_dir.clone(),
                work,
            ));
            workers.push(worker);
        }

        let ctx = Busy { workers };
        let state = ctx.into();

        Ok(state)
    }
}

impl State<Busy> {
    pub async fn update(
        mut self,
        events: &mut Vec<WorkerEvent>,
        runner: &mut dyn IWorkerRunner,
    ) -> Result<Updated> {
        for worker_slot in &mut self.ctx.workers {
            let worker = worker_slot.take().unwrap().update(events, runner).await?;

            worker_slot.replace(worker);
        }

        let updated = if self.all_workers_done() {
            let done = Done {
                cause: DoneCause::WorkersDone,
            };
            Updated::Done(done.into())
        } else {
            Updated::Busy(self)
        };

        Ok(updated)
    }

    fn all_workers_done(&self) -> bool {
        self.ctx
            .workers
            .iter()
            .all(|worker| worker.as_ref().unwrap().is_done())
    }

    pub async fn stop(mut self, task_id: TaskId) -> Result<Self> {
        self.ctx.workers =
            futures::future::try_join_all(self.ctx.workers.iter_mut().map(|worker| async move {
                match worker.take() {
                    Some(worker) => {
                        let mut new_worker = None;
                        if let Worker::Running(state) = worker {
                            if state.work().task_id == task_id {
                                new_worker = Some(Worker::Done(state.stop().kill().await?));
                            }
                        }
                        Ok::<std::option::Option<Worker>, anyhow::Error>(new_worker)
                    }
                    None => Ok(None),
                }
            }))
            .await?;

        Ok(self)
    }
}

pub enum Updated {
    Busy(State<Busy>),
    Done(State<Done>),
}

impl From<Updated> for Scheduler {
    fn from(updated: Updated) -> Self {
        match updated {
            Updated::Busy(state) => state.into(),
            Updated::Done(state) => state.into(),
        }
    }
}

impl State<Done> {
    pub fn cause(&self) -> DoneCause {
        self.ctx.cause.clone()
    }
}
