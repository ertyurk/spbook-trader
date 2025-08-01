// Monitoring and metrics service

pub struct MonitorService {
    name: String,
}

impl MonitorService {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}