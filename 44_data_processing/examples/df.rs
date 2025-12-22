use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::util::pretty::print_batches;
use datafusion::prelude::*;

#[tokio::main]
async fn main() -> datafusion::error::Result<()> {
    // register the table
    let mut ctx = ExecutionContext::new();
    ctx.register_csv("example", "fixtures/example.csv", CsvReadOptions::new())
        .await?;

    // create a plan to run a SQL query
    let df = ctx
        .sql("SELECT a, MIN(b) FROM example GROUP BY a LIMIT 100")
        .await?;

    // execute and print results
    df.show().await?;
    Ok(())
}
