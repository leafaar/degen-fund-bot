use base64::{self, Engine};
use bs58;
use colored::*;
use dotenv::dotenv;
use env_logger::Builder;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, LevelFilter};
use reqwest;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use std::env;
use std::io::Write;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logger configuration with colors
    Builder::new()
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Error => "ERROR".red(),
                log::Level::Warn => "WARN".yellow(),
                log::Level::Info => "INFO".green(),
                log::Level::Debug => "DEBUG".blue(),
                log::Level::Trace => "TRACE".cyan(),
            };
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .blue(),
                level,
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    info!("{}", "Starting Degen Fund Bot".bold());

    dotenv().ok();

    // Reading environment variables
    let rpc_url = env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set in .env");
    let private_key_base58 =
        env::var("PRIVATE_KEY_BASE58").expect("PRIVATE_KEY_BASE58 must be set in .env");
    let buy_amount = env::var("BUY_AMOUNT").expect("BUY_AMOUNT must be set in .env");
    let token_to_buy = env::var("TOKEN_TO_BUY").expect("TOKEN_TO_BUY must be set in .env");

    // Decode base58 private key
    let private_key = bs58::decode(private_key_base58).into_vec()?;
    let keypair = Keypair::from_bytes(&private_key)?;
    let buyer = keypair.pubkey().to_string();

    info!(
        "Buying {} tokens using wallet {}",
        buy_amount.yellow(),
        buyer.bright_green()
    );

    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Configure transaction URL
    let transaction_url = format!("https://www.degen.fund/api/antibot/{}", token_to_buy);
    let url = format!(
        "{}?&buy-amount={}&buyer={}",
        transaction_url, buy_amount, buyer
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Preparing transaction...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let response = reqwest::get(&url).await?.text().await?;
    let byte_tx = base64::engine::general_purpose::STANDARD.decode(&response)?;
    let mut tx: Transaction = bincode::deserialize(&byte_tx)?;

    // Sign the transaction
    let our_pubkey = keypair.pubkey();
    let our_signature_index = tx
        .message
        .account_keys
        .iter()
        .position(|&pubkey| pubkey == our_pubkey);

    if let Some(index) = our_signature_index {
        if tx.signatures[index] == Signature::default() {
            let message_data = tx.message_data();
            let signature = keypair.sign_message(&message_data);
            tx.signatures[index] = signature;
        }
    } else {
        return Err("Our public key is not in the list of signers"
            .red()
            .to_string()
            .into());
    }

    spinner.finish_with_message("Transaction prepared successfully!".green().to_string());

    // Send the transaction
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Sending transaction...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let signature = client.send_transaction(&tx)?;

    spinner.finish_with_message("Transaction sent successfully!".green().to_string());

    info!(
        "Transaction signature: {}",
        signature.to_string().bright_green()
    );

    let solscan_url = format!("https://solscan.io/tx/{}", signature);
    info!(
        "View transaction on Solscan: {}",
        solscan_url.bright_blue().underline()
    );

    Ok(())
}
