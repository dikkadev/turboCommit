use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DebugLogger {
    file: Option<File>,
    use_stdout: bool,
}

impl DebugLogger {
    pub fn new(debug_file: Option<String>) -> Self {
        let (file, use_stdout) = match debug_file {
            Some(path) if path == "-" => (None, true),
            Some(path) => (
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(Path::new(&path))
                    .ok(),
                false
            ),
            None => (None, false),
        };
        Self { file, use_stdout }
    }

    pub fn log(&mut self, category: &str, content: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        let log_line = format!("{};{};{}\n", timestamp, category, content);
        
        if self.use_stdout {
            let _ = io::stdout().write_all(log_line.as_bytes());
            let _ = io::stdout().flush();
        } else if let Some(file) = &mut self.file {
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }
    }

    pub fn log_request(&mut self, request_json: &str) {
        self.log("request", request_json);
    }

    pub fn log_response(&mut self, response_json: &str) {
        self.log("response", response_json);
    }

    pub fn log_error(&mut self, error: &str) {
        self.log("error", error);
    }

    pub fn log_info(&mut self, info: &str) {
        self.log("info", info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_debug_logger_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        
        let mut logger = DebugLogger::new(Some(path.clone()));
        logger.log_info("test message");
        
        let mut content = String::new();
        File::open(path).unwrap().read_to_string(&mut content).unwrap();
        
        assert!(content.contains("test message"));
        assert!(content.contains(";info;"));
    }

    #[test]
    fn test_debug_logger_none() {
        let mut logger = DebugLogger::new(None);
        logger.log_info("test message"); // Should not panic
    }
} 