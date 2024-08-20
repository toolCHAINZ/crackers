use anyhow::anyhow;
use async_openai::config::Config;
use async_openai::error::OpenAIError;
use async_openai::types::{ChatCompletionRequestUserMessageContent, CreateChatCompletionResponse};
use async_openai::Client;

use crate::agents::generic::{Agent, GenericAgent};
use crate::agents::model::Model;

pub enum AssemblyMode {
    First,
    Reflection,
}

impl AssemblyMode {
    pub fn get_system_prompt(&self) -> &'static str {
        match self {
            AssemblyMode::First => include_str!("first_shot_system.txt"),
            AssemblyMode::Reflection => include_str!("reflection_system.txt"),
        }
    }
}
pub struct AssemblyAgent<C: Config> {
    model: Model,
    agent: GenericAgent<C>,
}

impl<C: Config> AssemblyAgent<C> {
    pub fn new(c: Client<C>, model: Model) -> Self {
        let mode = AssemblyMode::First;
        let prompt = mode.get_system_prompt();
        Self {
            model,
            agent: GenericAgent::new(c, prompt, model),
        }
    }

    pub async fn code<T: Into<ChatCompletionRequestUserMessageContent>>(
        &mut self,
        message: T,
    ) -> anyhow::Result<String> {
        let msg = self.agent.chat(message).await?;
        let result = msg.choices[0]
            .message
            .clone()
            .content
            .ok_or(anyhow!("huh"))?;
        if result.starts_with("```") {
            if let Some(idx) = result.rfind("```") {
                if let Some(a) = result.find('\n') {
                    return Ok(result[a + 1..idx].to_string());
                }
            }
        }
        Ok(result)
    }

    pub fn reset_for_reflection(&mut self) {
        let client = self.agent.client().clone();
        self.agent = GenericAgent::new(
            client,
            AssemblyMode::Reflection.get_system_prompt(),
            self.model,
        );
    }
}

impl<C: Config> Agent for AssemblyAgent<C> {
    async fn chat<T: Into<ChatCompletionRequestUserMessageContent>>(
        &mut self,
        message: T,
    ) -> Result<CreateChatCompletionResponse, OpenAIError> {
        self.agent.chat(message).await
    }
}

#[cfg(test)]
mod tests {
    use async_openai::Client;

    use crate::agents::assembly::AssemblyAgent;
    use crate::agents::generic::Agent;
    use crate::agents::model::Model;

    #[tokio::test]
    async fn test_first_shot() {
        let mut agent = AssemblyAgent::new(Client::new(), Model::Gpt4o);
        agent
            .chat(include_str!("../../procedure/user.txt"))
            .await
            .unwrap();
    }
}
