use std::env;
use std::env::args;

use chatgpt::prelude::*;
use futures_util::StreamExt;
use std::io::{stdout, Write};
use chrono::Local;
use dotenv::dotenv;
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use chatgpt::client;
use chatgpt::types::Role;


//parse message json
fn parse_json(json_str: &str) -> Result<(String, i32, i32)> {
    let json_obj: Value = serde_json::from_str(json_str)?;

    let message = json_obj["message"].as_str().unwrap().to_string();
    let chat_id = json_obj["ChatId"].as_i64().unwrap() as i32;
    let user_id = json_obj["UserId"].as_i64().unwrap() as i32;

    Ok((message, chat_id, user_id))
}

/// Requires the `streams` crate feature
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    //TODO передать входящий json
    let json_str = r#"{"message":"Now could you do the same, but for the Java language","ChatId":123,"UserId":456}"#;

    let (message, chat_id, user_id) = parse_json(json_str).unwrap();
    //TODO получить json последнего conversation по chat_id из базы данных
    let json_conversation = r#"[{"role":"system","content":"You are ChatGPT, an AI model developed by OpenAI. Answer as concisely as possible. Today is: 03/06/2023 16:17"},
    {"role":"user","content":"Could you describe the Rust programming language in 5 words?"},
    {"role":"assistant","content":"Fast, safe, concurrent, modern, expressive."},
    {"role":"user","content":"Now could you do the same, but for the Zig language?"},
    {"role":"assistant","content":"Efficient, readable, robust, self-contained, low-level."}]"#;

    let mut messages: Vec<ChatMessage> = serde_json::from_str(json_conversation).unwrap();


    //remove log
    if !messages.is_empty() {
        messages.remove(0);
    }
    //add new log
    let new_message = ChatMessage {
        role: Role::System,
        content: format!("You are ChatGPT, an AI model developed by OpenAI.\
         Answer as concisely as possible. Today is: {0}", Local::now().format("%d/%m/%Y %H:%M")),
    };
    messages.insert(0, new_message);


    // Creating a client
    let key = env::var("OAI_TOKEN").unwrap();
    let client = ChatGPT::new(key)?;
    let mut conversation = client.new_conversation();
    conversation.history = messages;


    // Acquiring a streamed response
    // Note, that the `futures_util` crate is required for most
    // stream related utility methods
    let mut stream = conversation
        .send_message_streaming(message)
        .await?;

    // Iterating over a stream and collecting the results into a vector
    //TODO сейчас поток вывода выводится в консоль, передать его клиенту
    let mut output: Vec<ResponseChunk> = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            ResponseChunk::Content {
                delta,
                response_index,
            } => {
                // Printing part of response without the newline
                print!("{delta}");
                // Manually flushing the standard output, as `print` macro does not do that
                stdout().lock().flush().unwrap();
                output.push(ResponseChunk::Content {
                    delta,
                    response_index,
                });
            }
            other => output.push(other),
        }
    }

    // Parsing ChatMessage from the response chunks and saving it to the conversation history
    let messages = ChatMessage::from_response_chunks(output);
    conversation.history.push(messages[0].to_owned());
    //TODO обновленный conversation записывается в файл, добавить его в базу данных
    conversation
        .save_history_json("example_conversation.json")
        .await?;
    Ok(())
}
