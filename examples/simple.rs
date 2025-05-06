use metrics_actix_dashboard::hello_world;



#[tokio::main]
async fn main() {
    hello_world().await;
}
