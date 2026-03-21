use serde::Deserialize;

const MP_API: &str = "https://api.mercadopago.com";

pub struct MercadoPagoClient {
    pub access_token: String,
    pub terminal_id: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct MpOrderResponse {
    id: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct MpDeviceList {
    devices: Vec<MpDevice>,
}

#[derive(Deserialize)]
pub struct MpDevice {
    pub id: String,
    pub operating_mode: Option<String>,
}

impl MercadoPagoClient {
    pub fn new(access_token: String, terminal_id: String) -> Self {
        Self {
            access_token,
            terminal_id,
            client: reqwest::Client::new(),
        }
    }

    /// Lista los terminales Point vinculados a la cuenta.
    pub async fn list_terminals(&self) -> Result<Vec<MpDevice>, String> {
        let resp = self
            .client
            .get(format!("{MP_API}/terminals/v1/list"))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| format!("MP request failed: {e}"))?;

        let http_status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !http_status.is_success() {
            return Err(format!("MP API {http_status}: {text}"));
        }

        let list: MpDeviceList =
            serde_json::from_str(&text).map_err(|e| format!("MP parse error: {e} — {text}"))?;

        Ok(list.devices)
    }

    /// Activa el modo PDV en la terminal (bloqueado, solo acepta pagos por API).
    pub async fn set_pdv_mode(&self) -> Result<(), String> {
        let resp = self
            .client
            .patch(format!(
                "{MP_API}/point/integration-api/devices/{}",
                self.terminal_id
            ))
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({ "operating_mode": "PDV" }))
            .send()
            .await
            .map_err(|e| format!("MP request failed: {e}"))?;

        let http_status = resp.status();
        if !http_status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("MP API {http_status}: {text}"));
        }

        Ok(())
    }

    /// Crea una orden de pago en la terminal Point y devuelve el order_id.
    pub async fn create_order(
        &self,
        amount: f64,
        description: &str,
        external_ref: &str,
    ) -> Result<String, String> {
        let body = serde_json::json!({
            "type": "point",
            "external_reference": external_ref,
            "expiration_time": "PT2M",
            "description": description,
            "transactions": {
                "payments": [{ "amount": format!("{:.2}", amount) }]
            },
            "config": {
                "point": {
                    "terminal_id": self.terminal_id,
                    "print_on_terminal": "no_ticket"
                }
            }
        });

        let resp = self
            .client
            .post(format!("{MP_API}/v1/orders"))
            .bearer_auth(&self.access_token)
            .header("X-Idempotency-Key", uuid::Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("MP request failed: {e}"))?;

        let http_status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !http_status.is_success() {
            return Err(format!("MP API {http_status}: {text}"));
        }

        let order: MpOrderResponse =
            serde_json::from_str(&text).map_err(|e| format!("MP parse error: {e}"))?;

        order.id.ok_or_else(|| "MP response missing id".to_string())
    }

    /// Consulta el estado de una orden.
    /// Estados: "created" | "at_terminal" | "completed" | "expired" | "canceled"
    pub async fn get_order_status(&self, order_id: &str) -> Result<String, String> {
        let resp = self
            .client
            .get(format!("{MP_API}/v1/orders/{order_id}"))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| format!("MP request failed: {e}"))?;

        let http_status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !http_status.is_success() {
            return Err(format!("MP API {http_status}: {text}"));
        }

        let order: MpOrderResponse =
            serde_json::from_str(&text).map_err(|e| format!("MP parse error: {e}"))?;

        Ok(order.status.unwrap_or_else(|| "unknown".to_string()))
    }

    /// Cancela una orden pendiente.
    pub async fn cancel_order(&self, order_id: &str) -> Result<(), String> {
        let resp = self
            .client
            .delete(format!("{MP_API}/v1/orders/{order_id}"))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| format!("MP request failed: {e}"))?;

        let http_status = resp.status();
        if !http_status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("MP API {http_status}: {text}"));
        }

        Ok(())
    }
}
