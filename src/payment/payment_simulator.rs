pub struct PaymentResult {
    pub success: bool,
    pub method: String,
    pub message: String,
}

pub struct PaymentSimulator {
    simulation_mode: bool,
}

impl PaymentSimulator {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    pub fn process(&self, amount: f64) -> PaymentResult {
        if self.simulation_mode {
            log::info!("SIMULATION | Payment: processing ${amount:.2}");
            PaymentResult {
                success: true,
                method: "SIMULATED".to_string(),
                message: format!("Payment of ${amount:.2} simulated successfully"),
            }
        } else {
            // TODO: integrate real payment terminal
            log::warn!("PRODUCTION | Payment: real terminal not yet implemented");
            PaymentResult {
                success: false,
                method: String::new(),
                message: "Real payment terminal not implemented".to_string(),
            }
        }
    }
}
