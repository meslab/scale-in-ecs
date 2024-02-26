mod autoscaling;
mod ecs;
mod elasticache;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let as_client = autoscaling::initialize_client(&args.region).await;
    let asgs = autoscaling::list_asgs(&as_client, &args.cluster, 0).await?;
    info!("ASGs: {:?}", asgs);

    for asg in asgs {
        autoscaling::scale_down_asg(&as_client, &asg, 0).await?;
    }

    let elc_client = elasticache::initialize_client(&args.region).await;
    let replication_groups =
        elasticache::list_replication_groups(&elc_client, &args.cluster).await?;
    info!("Replication Groups: {:?}", replication_groups);

    for replication_group in replication_groups {
        elasticache::delete_replication_group(&elc_client, &replication_group).await?;
    }

    let ecs_client = ecs::initialize_client(&args.region).await;
    let services = ecs::get_service_arns(&ecs_client, &args.cluster, 0).await?;
    info!("Services: {:?}", services);

    for service in services {
        ecs::scale_down_service(&ecs_client, &args.cluster, &service, 0).await?;
    }

    debug!("Cluster: {} Region: {}.", &args.cluster, &args.region);

    Ok(())
}
