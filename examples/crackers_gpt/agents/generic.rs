use crate::agents::model::Model;
use async_openai::config::Config;
use async_openai::error::OpenAIError;
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPart, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
    ChatCompletionResponseMessage, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    CreateChatCompletionResponse,
};
use async_openai::Client;
use std::fmt::Debug;
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct GenericAgent<C: Config> {
    history: Vec<ChatCompletionRequestMessage>,
    model: Model,
    client: Client<C>,
}

impl<C: Config> GenericAgent<C> {
    pub fn new<S: Into<String>>(client: Client<C>, system_prompt: S, model: Model) -> Self {
        Self {
            client,
            history: vec![Self::build_system_prompt(system_prompt)],
            model,
        }
    }
    pub async fn chat<T: Into<ChatCompletionRequestUserMessageContent>>(
        &mut self,
        message: T,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        self.add_user_message(message);
        let req = self.build_request()?;

        let resp = self.client.chat().create(req).await?;
        self.add_assistant_message(&resp);
        Ok(resp)
    }

    #[allow(unused)]
    pub fn last_message(&self) -> Option<&ChatCompletionRequestMessage> {
        self.history.last()
    }

    #[allow(unused)]
    pub fn model(&self) -> Model {
        self.model
    }

    pub fn client(&self) -> &Client<C> {
        &self.client
    }
    fn add_user_message<T: Into<ChatCompletionRequestUserMessageContent>>(&mut self, msg: T) {
        let req = Self::build_user_prompt(msg);
        print_request(&req);
        self.history.push(req);
    }

    fn add_assistant_message(&mut self, msg: &CreateChatCompletionResponse) {
        let req = Self::build_assistant_prompt(&msg.choices[0].message);
        print_request(&req);
        self.history.push(req);
    }

    fn build_request(&self) -> Result<CreateChatCompletionRequest, OpenAIError> {
        CreateChatCompletionRequestArgs::default()
            .model(self.model.name())
            .messages(self.history.clone())
            .build()
    }

    fn build_system_prompt<S: Into<String>>(s: S) -> ChatCompletionRequestMessage {
        ChatCompletionRequestSystemMessage {
            content: s.into(),
            name: None,
        }
        .into()
    }

    fn build_assistant_prompt(
        resp: &ChatCompletionResponseMessage,
    ) -> ChatCompletionRequestMessage {
        ChatCompletionRequestAssistantMessageArgs::default()
            .content(resp.content.clone().unwrap())
            .build()
            .unwrap()
            .into()
    }

    fn build_user_prompt<T: Into<ChatCompletionRequestUserMessageContent>>(
        text: T,
    ) -> ChatCompletionRequestMessage {
        ChatCompletionRequestUserMessage {
            content: text.into(),
            name: None,
        }
        .into()
    }
}

pub trait Agent {
    // todo: maybe having this trait is overkill for what I'm doing. Consider removing.
    #[allow(unused)]
    async fn chat<T: Into<ChatCompletionRequestUserMessageContent>>(
        &mut self,
        message: T,
    ) -> Result<CreateChatCompletionResponse, OpenAIError>;
}

impl<C: Config> Agent for GenericAgent<C> {
    async fn chat<T: Into<ChatCompletionRequestUserMessageContent>>(
        &mut self,
        message: T,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        self.chat(message).await
    }
}

fn print_request(req: &ChatCompletionRequestMessage) {
    match req {
        ChatCompletionRequestMessage::System(sys) => {
            event!(Level::INFO, "\n(system):\n{}", sys.content)
        }
        ChatCompletionRequestMessage::User(u) => match &u.content {
            ChatCompletionRequestUserMessageContent::Text(t) => {
                event!(Level::INFO, "\n(user):\n{}", &t)
            }
            ChatCompletionRequestUserMessageContent::Array(a) => {
                event!(Level::INFO, "\n(user)");
                for x in a {
                    match x {
                        ChatCompletionRequestMessageContentPart::Text(t) => {
                            event!(Level::INFO, "{}", &t.text)
                        }
                        ChatCompletionRequestMessageContentPart::ImageUrl(i) => {
                            event!(Level::INFO, "{}", &i.image_url.url)
                        }
                    }
                }
            }
        },
        ChatCompletionRequestMessage::Assistant(a) => {
            if let Some(a) = &a.content {
                event!(Level::INFO, "\n(assistant):\n{}", &a)
            }
        }
        ChatCompletionRequestMessage::Tool(_) => {}
        ChatCompletionRequestMessage::Function(_) => {}
    }
}
