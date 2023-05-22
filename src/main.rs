use fake::{faker, Fake};
use rand::{prelude::Distribution, Rng};
use reqwest::ClientBuilder;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use tokio_cron_scheduler::{Job, JobScheduler};

struct DeviceIdDistribution;
impl Distribution<u8> for DeviceIdDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        const RANGE: u32 = 37;
        const GEN_DEVICE_ID_STR_CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz-";
        loop {
            let var = rng.next_u32() >> (32 - 6);
            if var < RANGE {
                return GEN_DEVICE_ID_STR_CHARSET[var as usize];
            }
        }
    }
}

fn device_id() -> String {
    rand::thread_rng()
        .sample_iter(&DeviceIdDistribution)
        .take(36)
        .map(char::from)
        .collect()
}

async fn send_message(user: &String, message: &String) -> Result<(), Box<dyn std::error::Error>> {
    let user_agent = faker::internet::en::UserAgent().fake::<String>();
    let device_id = device_id();

    let client = ClientBuilder::new().user_agent(user_agent).build()?;

    let mut map = HashMap::new();
    map.insert("username", user);
    map.insert("question", message);
    map.insert("deviceId", &device_id);

    let res = client
        .post("https://ngl.link/api/submit".to_owned())
        .form(&map)
        .send()
        .await?;

    let status = res.status();
    if !status.is_success() {
        return Err(format!("{}: {}", status, res.text().await?).into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}[2J", 27 as char);

    // ask for user
    let user = loop {
        let mut user = String::new();
        print!("Enter user: ");
        io::stdout().flush()?;
        std::io::stdin().read_line(&mut user)?;
        let user = user.trim().to_string();
        if !user.is_empty() {
            break user;
        }
    };

    print!("{}[2J", 27 as char);

    let message: Option<String> = loop {
        let mut message = String::new();
        print!("Enter message (leave empty for random generated junk): ");
        io::stdout().flush()?;
        std::io::stdin().read_line(&mut message)?;
        let message = message.trim().to_string();
        if !message.is_empty() {
            break Some(message);
        } else {
            break None;
        }
    };

    print!("{}[2J", 27 as char);

    let scheduler = JobScheduler::new().await?;

    // run under 25 times per minute (rate limit is exactly 25 times per minute)
    let job = Job::new_async("0/3 * * * * *", move |_uuid, mut _l| {
        let user = user.clone();
        let message = match &message {
            Some(message) => message.clone(),
            None => faker::lorem::en::Sentence(5..25).fake::<String>(),
        };
        Box::pin(async move {
            match send_message(&user, &message).await {
                Ok(_) => {
                    println!("[SENT:{}]: {}", &user, &message);
                }
                Err(e) => {
                    eprintln!("[ERROR:{}]: {}", &user, e);
                }
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    tokio::signal::ctrl_c().await?;

    Ok(())
}
