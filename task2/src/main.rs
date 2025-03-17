use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    system_instruction,
    transaction::Transaction,
};
use config::{Config, File};
use serde::Deserialize;
use std::{sync::Arc, time::{Duration, Instant}};
use tokio::{sync::Semaphore, task};
use futures::stream::{FuturesUnordered, StreamExt};

#[derive(Debug, Deserialize)]
struct AppConfig {
    from_wallets: Vec<String>,
    to_wallets: Vec<String>,
    amount: u64,
}

#[derive(Debug)]
struct TxMetrics {
    signature: String,
    send_time: Duration,
    finalization_time: Duration,
    total_time: Duration,
    finalized_successfully: bool,
}

/// Ожидание финализации транзакции с таймаутом.
/// Если транзакция финализируется до истечения timeout_duration,
/// функция возвращает (время ожидания, true). Иначе – (время ожидания, false).
async fn wait_for_finalization(client: Arc<RpcClient>, signature: &str) -> (Duration, bool) {
    let start = Instant::now();
    let timeout_duration = Duration::from_secs(30);
    let mut success = false;

    loop {
        if start.elapsed() > timeout_duration {
            break;
        }
        let statuses = client.get_signature_statuses(&[signature.parse().unwrap()]).await;
        if let Ok(statuses) = statuses {
            if let Some(Some(s)) = statuses.value.get(0) {
                success = s.status.is_ok();
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    (start.elapsed(), success)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Config::builder()
        .add_source(File::with_name("/home/ando/documents/other/test/task2/config.yaml"))
        .build()?;
    let config: AppConfig = settings.try_deserialize()?;

    if config.from_wallets.len() != config.to_wallets.len() {
        eprintln!("Количество кошельков-отправителей и получателей должно совпадать.");
        std::process::exit(1);
    }

    let rpc_url = "http://127.0.0.1:8899";
    let semaphore = Arc::new(Semaphore::new(5));
    let client = Arc::new(RpcClient::new(rpc_url.to_string()));

    let mut tasks = FuturesUnordered::new();

    for (from_path, to_addr) in config.from_wallets.iter().zip(config.to_wallets.iter()) {
        let keypair = read_keypair_file(&from_path)
            .unwrap_or_else(|_| panic!("Не удалось прочитать файл ключей: {}", from_path));
        let from_pubkey = keypair.pubkey();
        let to_pubkey = to_addr.parse::<Pubkey>()
            .expect("Неверный формат публичного ключа получателя");
        let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, config.amount);

        let client = Arc::clone(&client);
        let sem = Arc::clone(&semaphore);

        tasks.push(task::spawn(async move {
            let _permit = sem.acquire_owned().await;
            let recent_blockhash = client.get_latest_blockhash().await
                .expect("Не удалось получить последний блокхэш");

            let tx = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&from_pubkey),
                &[&keypair],
                recent_blockhash,
            );

            let start_send = Instant::now();
            let signature = client.send_transaction(&tx).await
                .expect("Ошибка отправки транзакции");
            let send_time = start_send.elapsed();

            let (finalization_time, finalized_successfully) =
                wait_for_finalization(Arc::clone(&client), &signature.to_string()).await;
            let total_time = send_time + finalization_time;

            TxMetrics {
                signature: signature.to_string(),
                send_time,
                finalization_time,
                total_time,
                finalized_successfully,
            }
        }));
    }

    let mut metrics = Vec::new();

    println!("Результаты транзакций:");
    while let Some(result) = tasks.next().await {
        match result {
            Ok(tx_metrics) => {
                println!(
                    "Tx Hash: {}\tSend Time: {:.2?}\tFinalization Time: {:.2?}\tTotal Time: {:.2?}\tFinalized: {}",
                    tx_metrics.signature,
                    tx_metrics.send_time,
                    tx_metrics.finalization_time,
                    tx_metrics.total_time,
                    if tx_metrics.finalized_successfully { "Да" } else { "Нет" }
                );
                metrics.push(tx_metrics);
            },
            Err(e) => {
                eprintln!("Ошибка задачи: {:?}", e);
            }
        }
    }

    let total_txs = metrics.len();
    let successful_txs = metrics.iter().filter(|m| m.finalized_successfully).count();
    let success_percentage = if total_txs > 0 {
        (successful_txs as f64 / total_txs as f64) * 100.0
    } else {
        0.0
    };

    if !metrics.is_empty() {
        let (mut total_send, mut total_final, mut total_total) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
        let (mut min_send, mut max_send) = (metrics[0].send_time, metrics[0].send_time);
        let (mut min_final, mut max_final) = (metrics[0].finalization_time, metrics[0].finalization_time);
        let (mut min_total, mut max_total) = (metrics[0].total_time, metrics[0].total_time);

        for m in &metrics {
            total_send += m.send_time;
            total_final += m.finalization_time;
            total_total += m.total_time;

            if m.send_time < min_send { min_send = m.send_time; }
            if m.send_time > max_send { max_send = m.send_time; }
            if m.finalization_time < min_final { min_final = m.finalization_time; }
            if m.finalization_time > max_final { max_final = m.finalization_time; }
            if m.total_time < min_total { min_total = m.total_time; }
            if m.total_time > max_total { max_total = m.total_time; }
        }

        let count = metrics.len() as u32;
        let avg_send = total_send / count;
        let avg_final = total_final / count;
        let avg_total = total_total / count;

        println!("\nСтатистика времени отправки транзакций:");
        println!("Минимальное время: {:.2?}", min_send);
        println!("Максимальное время: {:.2?}", max_send);
        println!("Среднее время: {:.2?}", avg_send);

        println!("\nСтатистика времени финализации транзакций:");
        println!("Минимальное время: {:.2?}", min_final);
        println!("Максимальное время: {:.2?}", max_final);
        println!("Среднее время: {:.2?}", avg_final);

        println!("\nОбщая статистика времени транзакций:");
        println!("Минимальное время: {:.2?}", min_total);
        println!("Максимальное время: {:.2?}", max_total);
        println!("Среднее время: {:.2?}", avg_total);

        println!("\nПроцент успешно финализированных транзакций: {:.2}%", success_percentage);
    }

    Ok(())
}
