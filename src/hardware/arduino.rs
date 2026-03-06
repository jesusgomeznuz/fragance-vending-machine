pub struct Arduino {
    simulation_mode: bool,
}

impl Arduino {
    pub fn new(simulation_mode: bool) -> Self {
        Self { simulation_mode }
    }

    pub fn send_dispense_command(&self, product_id: i64) -> bool {
        if self.simulation_mode {
            log::info!("SIMULATION | Arduino: dispense command sent for product_id={product_id}");
            true
        } else {
            // TODO: open serial port and send command to real Arduino
            log::warn!("PRODUCTION | Arduino: real hardware communication not yet implemented");
            false
        }
    }
}
