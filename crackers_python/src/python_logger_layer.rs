use pyo3::prelude::*;
// PythonLoggerLayer: tracing subscriber layer for forwarding Rust logs to Python logging
use pyo3::Python;
use pyo3::types::{PyDict, PyModule};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

pub struct PythonLoggerLayer;

impl<S> Layer<S> for PythonLoggerLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        struct MessageVisitor {
            message: Option<String>,
        }
        impl tracing_subscriber::field::Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = Some(format!("{:?}", value));
                }
            }
        }
        Python::attach(|py| {
            if let Ok(logging) = PyModule::import(py, "logging") {
                let meta = event.metadata();
                let module_path = meta.module_path().unwrap_or("");
                let file = meta.file().unwrap_or("");
                let line_num = meta.line().unwrap_or(0);
                let level = meta.level();
                let mut visitor = MessageVisitor { message: None };
                event.record(&mut visitor);
                let message = visitor.message.unwrap_or_default();
                // Build an `extra` dict so Python logging receives tracing metadata separately
                let extra = PyDict::new(py);
                let _ = extra.set_item("rust_file", file);
                let _ = extra.set_item("rust_line", line_num);
                // Optionally include module path so Python loggers can use it (e.g. as logger name)
                let _ = extra.set_item("rust_module", module_path);
                let kwargs = PyDict::new(py);
                let _ = kwargs.set_item("extra", extra);
                let py_level = match *level {
                    Level::ERROR => "error",
                    Level::WARN => "warning",
                    Level::INFO => "info",
                    Level::DEBUG => "debug",
                    Level::TRACE => "debug",
                };
                // Call the Python logging API with the message and `extra` metadata instead of embedding file info into the message
                let _ = logging.call_method(py_level, (message,), Some(&kwargs));
            }
        });
    }
}
