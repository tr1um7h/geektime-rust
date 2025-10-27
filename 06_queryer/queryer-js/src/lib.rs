use neon::prelude::*;

pub fn example_sql(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string(queryer::example_sql()))
}

pub fn query(mut cx: FunctionContext) -> JsResult<JsString> {
    let sql = cx.argument::<JsString>(0)?.value(&mut cx);
    let output_fmt = cx.argument_opt(1)
        .map_or(
            "csv".to_owned(),
            |v| v.to_string(&mut cx).unwrap().value(&mut cx),
        );

    if output_fmt != "csv" {
        return cx.throw_error("Only support csv in the moment");
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut data = rt.block_on(async { queryer::query(sql).await.unwrap() });

    Ok(cx.string(data.to_csv().unwrap()))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("query", query)?;
    cx.export_function("example_sql", example_sql)?;
    Ok(())
}
