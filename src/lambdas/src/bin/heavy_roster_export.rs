use lambda_runtime::Error;
use scintilla_route_lambdas::{run, RouteSpec};

const ROUTE: RouteSpec = RouteSpec::new(
    "heavy_roster_export",
    "POST",
    "/api/heavy/roster-export",
    "cohort_id",
);

#[tokio::main]
async fn main() -> Result<(), Error> {
    return run(ROUTE).await;
}
