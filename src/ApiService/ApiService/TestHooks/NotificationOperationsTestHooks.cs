﻿using System.Net;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.Azure.Functions.Worker;
using Microsoft.Azure.Functions.Worker.Http;
using Microsoft.Extensions.Logging;
using Microsoft.OneFuzz.Service;
using Microsoft.OneFuzz.Service.OneFuzzLib.Orm;

#if DEBUG
namespace ApiService.TestHooks {
    public class NotificationOperationsTestHooks {

        private readonly ILogger _log;
        private readonly IConfigOperations _configOps;
        private readonly INotificationOperations _notificationOps;

        public NotificationOperationsTestHooks(ILogger<NotificationOperationsTestHooks> log, IConfigOperations configOps, INotificationOperations notificationOps) {
            _log = log;
            _log.AddTag("TestHooks", nameof(NotificationOperationsTestHooks));
            _configOps = configOps; ;
            _notificationOps = notificationOps;
        }

        [Function("NewFilesTestHook")]
        public async Task<HttpResponseData> NewFiles([HttpTrigger(AuthorizationLevel.Anonymous, "put", Route = "testhooks/notificationOperations/newFiles")] HttpRequestData req) {
            _log.LogInformation("new files");
            var query = UriExtension.GetQueryComponents(req.Url);

            var container = query["container"];
            var fileName = query["fileName"];
            var isLastRetryAttempt = UriExtension.GetBool("isLastRetryAttempt", query, true);

            await _notificationOps.NewFiles(Container.Parse(container), fileName, isLastRetryAttempt);
            var resp = req.CreateResponse(HttpStatusCode.OK);
            return resp;
        }

        [Function("GetNotificationsTestHook")]
        public async Task<HttpResponseData> GetNotifications([HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "testhooks/notificationOperations/getNotifications")] HttpRequestData req) {
            _log.LogInformation("get notifications");

            var s = await req.ReadAsStringAsync();

            var query = UriExtension.GetQueryComponents(req.Url);
            var container = query["container"];
            var notifications = _notificationOps.GetNotifications(Container.Parse(container));

            var json = JsonSerializer.Serialize(await notifications.ToListAsync(), EntityConverter.GetJsonSerializerOptions());
            var resp = req.CreateResponse(HttpStatusCode.OK);
            await resp.WriteStringAsync(json);
            return resp;
        }

        [Function("GetQueueTasksTestHook")]
        public async Task<HttpResponseData> GetQueueTasksTestHook([HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "testhooks/notificationOperations/getQueueTasks")] HttpRequestData req) {

            _log.LogInformation("get queue tasks");
            var queueuTasks = _notificationOps.GetQueueTasks();

            var json = JsonSerializer.Serialize(await queueuTasks.ToListAsync(), EntityConverter.GetJsonSerializerOptions());
            var resp = req.CreateResponse(HttpStatusCode.OK);
            await resp.WriteStringAsync(json);
            return resp;
        }


        [Function("GetRegressionReportTaskTestHook")]
        public async Task<HttpResponseData> GetRegressionReportTask([HttpTrigger(AuthorizationLevel.Anonymous, "post", Route = "testhooks/notificationOperations/getRegressionReportTask")] HttpRequestData req) {
            _log.LogInformation("get regression report task");

            var s = await req.ReadAsStringAsync();
            var report = JsonSerializer.Deserialize<RegressionReport>(s!, EntityConverter.GetJsonSerializerOptions());
            var task = (_notificationOps as NotificationOperations)!.GetRegressionReportTask(report!);

            var json = JsonSerializer.Serialize(task, EntityConverter.GetJsonSerializerOptions());
            var resp = req.CreateResponse(HttpStatusCode.OK);
            await resp.WriteStringAsync(json);
            return resp;
        }
    }
}
#endif
