use serde::{Serialize, Deserialize};
use chrono::{Duration, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub subscriber: String,
    pub service: String,
    pub amount: f64,
    pub duration: String, // e.g., "30d"
    pub start_date: i64,  // UNIX timestamp
    pub end_date: i64,    // UNIX timestamp
    pub is_active: bool,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(subscriber: String, service: String, amount: f64, duration: String) -> Self {
        let duration_days = Self::parse_duration(&duration).unwrap_or(30); // Default to 30 days
        let start_date = Utc::now().timestamp();
        let end_date = (Utc::now() + Duration::days(duration_days)).timestamp();

        Subscription {
            id: uuid::Uuid::new_v4().to_string(),
            subscriber,
            service,
            amount,
            duration,
            start_date,
            end_date,
            is_active: true,
        }
    }

    /// Renew the subscription
    pub fn renew(&self) -> Self {
        if self.is_active {
            let duration_days = Self::parse_duration(&self.duration).unwrap_or(30);
            let new_end_date = (Utc::now() + Duration::days(duration_days)).timestamp();

            Subscription {
                id: uuid::Uuid::new_v4().to_string(),
                subscriber: self.subscriber.clone(),
                service: self.service.clone(),
                amount: self.amount,
                duration: self.duration.clone(),
                start_date: self.end_date, // Renewal starts when the current one ends
                end_date: new_end_date,
                is_active: true,
            }
        } else {
            panic!("Cannot renew an inactive subscription!");
        }
    }

    /// Cancel the subscription
    pub fn cancel(&mut self) {
        self.is_active = false;
    }

    /// Check if the subscription is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.end_date
    }

    /// Parse duration string into days
    fn parse_duration(duration: &str) -> Option<i64> {
        if duration.ends_with("d") {
            duration.trim_end_matches("d").parse::<i64>().ok()
        } else if duration.ends_with("w") {
            duration
                .trim_end_matches("w")
                .parse::<i64>()
                .ok()
                .map(|weeks| weeks * 7)
        } else if duration.ends_with("m") {
            duration
                .trim_end_matches("m")
                .parse::<i64>()
                .ok()
                .map(|months| months * 30) // Approximate months as 30 days
        } else {
            None
        }
    }
}
