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

impl MercadoPagoClient {
    pub fn new(access_token: String, terminal_id: String) -> Self {
        Self {
            access_token,
            terminal_id,
            client: reqwest::Client::new(),
        }
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
            "expiration_time": "PT10M",
            "description": description,
            "transactions": {
                "payments": [{ "amount": format!("{:.2}", amount) }]
            },
            "config": {
                "point": {
                    "terminal_id": self.terminal_id,
                    "print_on_terminal": "no_ticket"
                },
                "payment_method": {
                    "default_type": "credit_card"
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

    /// Consulta el estado de una orden. Devuelve el status string de MP:
    /// "created" | "at_terminal" | "processed" | "failed" | "expired" | "canceled"
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
}
