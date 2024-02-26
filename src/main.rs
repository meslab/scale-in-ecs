use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_autoscaling::config::Region as ASRegion;
use aws_sdk_autoscaling::{Client as ASClient, Config as ASConfig};
use aws_sdk_ecs::config::Region as EcsRegion;
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use aws_sdk_elasticache::config::Region as ElcRegion;
use aws_sdk_elasticache::{Client as ElcClient, Config as ElcConfig};
use clap::Parser;
use log::{debug, info};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.0.1",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Counts wwords frequency in a text file"
)]
struct Args {
    #[clap(short, long, default_value = "direc-prod-lb")]
    cluster: String,

    #[clap(short, long, default_value = "eu-central-1")]
    region: String,
}

async fn list_asgs(
    as_client: &ASClient,
    cluster: &String,
    desired_capacity: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut asgs = Vec::new();
    let mut asg_stream = as_client
        .describe_auto_scaling_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(asg) = asg_stream.next().await {
        debug!("ASG: {:?}", asg);
        for group in asg.unwrap().auto_scaling_groups.unwrap() {
            if group
                .auto_scaling_group_name
                .clone()
                .unwrap()
                .contains(cluster)
                && group
                    .desired_capacity
                    .clone()
                    .unwrap()
                    .gt(&desired_capacity)
            {
                asgs.push(group.auto_scaling_group_name.unwrap());
            }
        }
    }
    Ok(asgs)
}

async fn scale_down_asg(
    as_client: &ASClient,
    asg_name: &String,
    desired_capacity: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    as_client
        .update_auto_scaling_group()
        .auto_scaling_group_name(asg_name)
        .desired_capacity(desired_capacity)
        .min_size(desired_capacity)
        .max_size(desired_capacity)
        .send()
        .await?;
    Ok(())
}

async fn list_replication_groups(
    elc_client: &ElcClient,
    cluster: &String,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut replication_groups = Vec::new();
    let mut replication_groups_stream = elc_client
        .describe_replication_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(replication_group) = replication_groups_stream.next().await {
        debug!("Replication Groups: {:?}", replication_group);
        for group in replication_group.unwrap().replication_groups.unwrap() {
            if group
                .replication_group_id
                .clone()
                .unwrap()
                .contains(cluster)
                && group.status.clone().unwrap().contains("available")
            {
                replication_groups.push(group.replication_group_id.unwrap());
            }
        }
    }
    Ok(replication_groups)
}

async fn delete_replication_group(
    elc_client: &ElcClient,
    replication_group_id: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    elc_client
        .delete_replication_group()
        .replication_group_id(replication_group_id)
        .send()
        .await?;
    Ok(())
}

async fn get_service_arns(
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

async fn scale_down_service(
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let as_region = ASRegion::new(args.region.clone());
    let as_credentials_provider = DefaultCredentialsChain::builder()
        .region(as_region.clone())
        .build()
        .await;
    let as_config = ASConfig::builder()
        .credentials_provider(as_credentials_provider)
        .region(as_region)
        .build();
    let as_client = ASClient::from_conf(as_config);

    let asgs = list_asgs(&as_client, &args.cluster, 0).await?;
    info!("ASGs: {:?}", asgs);

    for asg in asgs {
        scale_down_asg(&as_client, &asg, 0).await?;
    }

    let elc_region = ElcRegion::new(args.region.clone());
    let elc_credentials_provider = DefaultCredentialsChain::builder()
        .region(elc_region.clone())
        .build()
        .await;
    let elc_config = ElcConfig::builder()
        .credentials_provider(elc_credentials_provider)
        .region(elc_region)
        .build();
    let elc_client = ElcClient::from_conf(elc_config);

    let replication_groups = list_replication_groups(&elc_client, &args.cluster).await?;
    info!("Replication Groups: {:?}", replication_groups);

    for replication_group in replication_groups {
        delete_replication_group(&elc_client, &replication_group).await?;
    }

    let ecs_region = EcsRegion::new(args.region.clone());
    let ecs_credentials_provider = DefaultCredentialsChain::builder()
        .region(ecs_region.clone())
        .build()
        .await;
    let ecs_config = EcsConfig::builder()
        .credentials_provider(ecs_credentials_provider)
        .region(ecs_region)
        .build();

    let ecs_client = EcsClient::from_conf(ecs_config);

    let services = get_service_arns(&ecs_client, &args.cluster, 0).await?;
    info!("Services: {:?}", services);

    for service in services {
        scale_down_service(&ecs_client, &args.cluster, &service, 0).await?;
    }

    debug!("Cluster: {} Region: {}.", &args.cluster, &args.region);

    Ok(())
}
