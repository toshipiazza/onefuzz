﻿using System;
using Azure.Core;
using Azure.ResourceManager.Storage;
using Microsoft.ApplicationInsights.DataContracts;
using Microsoft.OneFuzz.Service;

namespace IntegrationTests.Fakes;

public sealed class TestServiceConfiguration : IServiceConfig {
    public TestServiceConfiguration(string tablePrefix) {
        OneFuzzStoragePrefix = tablePrefix;
    }

    public string OneFuzzStoragePrefix { get; }

    public ResourceIdentifier OneFuzzFuncStorage { get; } =
        // not used by test code, this is a placeholder value
        StorageAccountResource.CreateResourceIdentifier(Guid.NewGuid().ToString(), "resource-group", "account-name");

    public string OneFuzzVersion => "9999.0.0"; // very big version to pass any >= checks

    public string? ApplicationInsightsAppId { get; set; } = "TestAppInsightsAppId";

    public string? ApplicationInsightsInstrumentationKey { get; set; } = "TestAppInsightsInstrumentationKey";

    public string? OneFuzzTelemetry => "TestOneFuzzTelemetry";

    public string? CliAppId => "TestGuid";

    public string? Authority => "TestAuthority";

    public string? TenantDomain => "TestDomain";
    public string? MultiTenantDomain => null;

    public string OneFuzzInstanceName => "UnitTestInstance";

    public string OneFuzzKeyvault => "TestOneFuzzKeyVault";
    public Uri OneFuzzInstance { get; } = new Uri("https://onefuzz-integration-test.example.com");

    // -- Remainder not implemented --

    public Uri? OneFuzzEndpoint => null;

    public LogDestination[] LogDestinations { get => throw new System.NotImplementedException(); set => throw new System.NotImplementedException(); }

    public SeverityLevel LogSeverityLevel => throw new System.NotImplementedException();

    public string? OneFuzzMonitor => throw new System.NotImplementedException();

    public string? OneFuzzOwner => throw new System.NotImplementedException();

    public ResourceIdentifier OneFuzzDataStorage => throw new NotImplementedException();

    public ResourceIdentifier OneFuzzResourceGroup => throw new NotImplementedException();

    public string? OneFuzzAllowOutdatedAgent => throw new NotImplementedException();
    public string? AppConfigurationEndpoint => throw new NotImplementedException();
    public string? AppConfigurationConnectionString => throw new NotImplementedException();
}
