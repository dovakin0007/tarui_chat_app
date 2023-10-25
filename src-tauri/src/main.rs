// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::Manager;
use std::{convert::Infallible, io::Write};
use llm::conversation_inference_callback;
struct AppState {
    model: Mutex<Box<dyn llm::Model>>,
}



pub struct MyChatState {
    my_chat: Mutex<String>
}


#[tauri::command(rename_all = "snake_case")]
async fn chat(message: String, app_handle: tauri::AppHandle) -> String{
    let app_state = app_handle.state::<AppState>();
    let chat_state = app_handle.state::<MyChatState>();
    let mut output = String::new();
    let model = app_state.inner().model.lock().unwrap();
    let mut history = chat_state.inner().my_chat.lock().unwrap();
    let inference_parameters = llm::InferenceParameters::default();
    let mut session = model.start_session(Default::default());
    session
        .feed_prompt(
            model.as_ref(),
            format!("A chat between a human and an assistant. \n{history}").as_str(),
            &mut Default::default(),
            llm::feed_prompt_callback(|resp| match resp {
                llm::InferenceResponse::PromptToken(t)
                | llm::InferenceResponse::InferredToken(t) => {
                    print_token(t);

                    Ok::<llm::InferenceFeedback, Infallible>(llm::InferenceFeedback::Continue)
                }
                _ => Ok(llm::InferenceFeedback::Continue),
            }),
        )
        .expect("Failed to ingest initial prompt.");


    let mut rng = rand::thread_rng();
    let mut res = llm::InferenceStats::default();

  
        println!();
        println!("### Human: {}\n", message.clone());
        print!("### Assistant:");

        let stats = session
                .infer::<Infallible>(
                model.as_ref(),
                &mut rng,
                &llm::InferenceRequest {
                    prompt: format!("### Human: {}\n### Assistant:", message.clone())
                        .as_str()
                        .into(),
                    parameters: &inference_parameters,
                    play_back_previous_tokens: false,
                    maximum_token_count: None,
                 },
               &mut Default::default(),
                conversation_inference_callback(&format!("### Assistant:"), |t|{
                    output.push_str(&t);
                    print_token(t)
                }
            ))
            .unwrap_or_else(|e| panic!("{e}"));

            res.feed_prompt_duration = res
                .feed_prompt_duration
                .saturating_add(stats.feed_prompt_duration);
            res.prompt_tokens += stats.prompt_tokens;
            res.predict_duration = res.predict_duration.saturating_add(stats.predict_duration);
            res.predict_tokens += stats.predict_tokens;
            
 
    history.push_str(&format!("### Human: {}\n ### Assistant: {}",message.clone(), output.clone()));

    // println!("\n\nInference stats:\n{res}");
    return  output;
}

fn print_token(t: String) {
    print!("{t}");
    std::io::stdout().flush().unwrap();

}

fn main() {
    tauri::Builder::default().setup(|app|{

        Ok({
        let model = llm::load_dynamic(
            Some(llm::ModelArchitecture::Llama),
            std::path::Path::new("C:\\Users\\dovak\\Downloads\\Wizard-Vicuna-7B-Uncensored.ggmlv3.q2_K.bin"),
            llm::TokenizerSource::Embedded,
            Default::default(),
            llm::load_progress_callback_stdout,
        )
        .unwrap_or_else(|err| {
            panic!("Failed to load model: {err}")
        });
    
        app.manage(MyChatState{
             my_chat: Mutex::from(String::from("
             ### Assistant: Hello - How may I help you today?\n\
             ### Human: What is the capital of France?\n\
             ### Assistant:  Paris is the capital of France."))
        });
        app.manage(AppState {
            model: Mutex::from(model),
        });
    })})
        .invoke_handler(tauri::generate_handler![chat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
