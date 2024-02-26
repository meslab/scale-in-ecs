use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_ecs::config::Region as EcsRegion;
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use log::debug;

pub async fn initialize_ecs_client(region: &str) -> EcsClient {
    let ecs_region = EcsRegion::new(region.to_owned());
    let ecs_credentials_provider = DefaultCredentialsChain::builder()
        .region(ecs_region.clone())
        .build()
        .await;
    let ecs_config = EcsConfig::builder()
        .credentials_provider(ecs_credentials_provider)
        .region(ecs_region)
        .build();
    EcsClient::from_conf(ecs_config)
}

pub async fn get_service_arns(
    ecs_client: &EcsClient,
    cluster: &String,
    desired_count: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut service_arns: Vec<String> = Vec::new();
    let mut ecs_services_stream = ecs_client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = ecs_services_stream.next().await {
        debug!("Services: {:?}", services);
        for service_arn in services.unwrap().service_arns.unwrap() {
            debug!("Service ARN: {:?}", service_arn);
            if service_arn.contains(cluster) {
                debug!("Service ARN: {}", service_arn);
                match ecs_client
                    .describe_services()
                    .cluster(cluster)
                    .services(service_arn.clone())
                    .send()
                    .await
                {
                    Ok(service) => {
                        debug!("Service: {:?}", service);
                        if service
                            .services
                            .unwrap()
                            .first()
                            .unwrap()
                            .desired_count
                            .gt(&desired_count)
                        {
                            service_arns.push(service_arn);
                        }
                    }
                    Err(e) => {
                        debug!("Error: {:?}", e);
                    }
                }
            }
        }
    }
    Ok(service_arns)
}

pub async fn scale_down_service(
    ecs_client: &EcsClient,
    cluster: &String,
    service_arn: &String,
    desired_count: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    ecs_client
        .update_service()
        .cluster(cluster)
        .service(service_arn)
        .desired_count(desired_count)
        .send()
        .await?;
    Ok(())
}
