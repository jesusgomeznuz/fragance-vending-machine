use crate::hardware::arduino::Arduino;

pub struct Dispenser {
    arduino: Arduino,
}

impl Dispenser {
    pub fn new(simulation_mode: bool) -> Self {
        Self {
            arduino: Arduino::new(simulation_mode),
        }
    }

    pub fn dispense(&self, product_id: i64) -> bool {
        log::info!("Dispenser: dispensing product_id={product_id}");
        self.arduino.send_dispense_command(product_id)
    }
}
