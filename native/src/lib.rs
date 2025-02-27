#[macro_use]
extern crate napi_derive;
#[macro_use]
extern crate lazy_static;

use command::{external::FrontendMessage, internal::Internal};
use crossbeam_channel::{unbounded, Sender};
use log::info;
use napi::{
  CallContext, JsExternal, JsFunction, JsObject, JsString, JsUndefined, JsUnknown, Result,
};
use shell::{Cell, CellChannel, CellProps, ServerMessage};
use std::thread;
use suggestions::Suggestions;
mod command;
mod logger;
mod shell;
mod suggestions;
mod util;

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("api", api)?;

  exports.create_named_method("getSuggestions", get_suggestions)?;

  exports.create_named_method("runCell", run_cell)?;

  exports.create_named_method("frontendMessage", frontend_message)?;

  logger::init().unwrap();

  Ok(())
}

#[js_function(1)]
fn api(ctx: CallContext) -> napi::Result<JsString> {
  let command: String = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;

  info!("Api call: {}", command);

  let result = if let Some(internal) = Internal::parse(&command) {
    internal.api()
  } else {
    "Internal command not found".to_string()
  };

  ctx.env.create_string(&result)
}

#[js_function(2)]
fn get_suggestions(ctx: CallContext) -> napi::Result<JsObject> {
  let value = ctx.get::<JsString>(0)?.into_utf8()?.into_owned()?;
  let current_dir = ctx.get::<JsString>(1)?.into_utf8()?.into_owned()?;

  info!("Getting suggestions for {}", value);

  let suggestions = Suggestions(current_dir, value);

  ctx.env.spawn(suggestions).map(|a| a.promise_object())
}

#[js_function(5)]
fn run_cell(ctx: CallContext) -> napi::Result<JsExternal> {
  let props: CellProps = ctx.env.from_js_value(ctx.get::<JsUnknown>(0)?)?;
  let server_message = ctx.get::<JsFunction>(1)?;

  let (sender, receiver) = unbounded::<CellChannel>();
  let external_sender = sender.clone();

  let tsfn = ctx.env.create_threadsafe_function(
    &server_message,
    0,
    |ctx: napi::threadsafe_function::ThreadSafeCallContext<Vec<ServerMessage>>| {
      ctx
        .value
        .iter()
        .map(|arg| ctx.env.to_js_value(&arg))
        .collect::<Result<Vec<JsUnknown>>>()
    },
  )?;

  info!("Running cell: {:?}", props);

  thread::spawn(move || {
    let cell = Cell::new(props.clone(), tsfn, sender, receiver);

    cell.run();

    info!("Finished running cell: {:?}", props);
  });

  ctx.env.create_external(external_sender)
}

#[js_function(2)]
fn frontend_message(ctx: CallContext) -> napi::Result<JsUndefined> {
  let attached_obj = ctx.get::<JsExternal>(0)?;
  let sender = ctx
    .env
    .get_value_external::<Sender<CellChannel>>(&attached_obj)?;

  let message: FrontendMessage = ctx.env.from_js_value(ctx.get::<JsUnknown>(1)?)?;

  info!("Frontend message: {:?}", message);

  if let Err(err) = sender.send(CellChannel::FrontendMessage(message)) {
    info!("Failed to send key: {}", err);
  }

  ctx.env.get_undefined()
}
