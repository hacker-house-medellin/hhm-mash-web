use lambda_runtime::Error;
use scintilla_route_lambdas::{run, RouteSpec};

const ROUTE: RouteSpec = RouteSpec::new(
    "heavy_application_score",
    "POST",
    "/api/heavy/application-score",
    "application_id",
);

#[tokio::main]
async fn main() -> Result<(), Error> {
    return run(ROUTE).await;
}
