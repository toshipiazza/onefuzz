param location string

var suffix = uniqueString(resourceGroup().id)
var appConfigName = 'app-config-${suffix}'

resource featureFlags 'Microsoft.AppConfiguration/configurationStores@2022-05-01' = {
  name: appConfigName
  location: location
  sku: {
    name: 'standard'
  }
}

resource configStoreFeatureflag 'Microsoft.AppConfiguration/configurationStores/keyValues@2021-10-01-preview' = {
  parent: featureFlags
  name: '.appconfig.featureflag~2FRenderOnlyScribanTemplates'
  properties: {
    value: string({
      id: 'RenderOnlyScribanTemplates'
      description: 'Render notification templates with scriban only'
      enabled: false
    })
    contentType: 'application/vnd.microsoft.appconfig.ff+json;charset=utf-8'
  }
}

resource validateNotificationConfigSemantics 'Microsoft.AppConfiguration/configurationStores/keyValues@2021-10-01-preview' = {
  parent: featureFlags
  name: '.appconfig.featureflag~2FSemanticNotificationConfigValidation'
  properties: {
    value: string({
      id: 'SemanticNotificationConfigValidation'
      description: 'Check notification configs for valid PATs and fields'
      enabled: true
    })
    contentType: 'application/vnd.microsoft.appconfig.ff+json;charset=utf-8'
  }
}

resource enableCustomMetricFeatureFlag 'Microsoft.AppConfiguration/configurationStores/keyValues@2021-10-01-preview' = {
  parent: featureFlags
  name: '.appconfig.featureflag~2FEnableCustomMetricTelemetry'
  properties: {
    value: string({
      id: 'EnableCustomMetricTelemetry'
      description: 'Allow custom metrics to be sent.'
      enabled: true
    })
    contentType: 'application/vnd.microsoft.appconfig.ff+json;charset=utf-8'
  }
}

output AppConfigEndpoint string = 'https://${appConfigName}.azconfig.io'
