use async_openai::config::Config;
use async_openai::Client;

use crate::agents::generic::GenericAgent;
use crate::agents::model::Model;

pub struct ReflectionAgent<C: Config> {
    agent: GenericAgent<C>,
}

impl<C: Config> ReflectionAgent<C> {
    pub fn new(c: Client<C>, model: Model) -> Self {
        Self {
            agent: GenericAgent::new(c, include_str!("system.txt"), model),
        }
    }

    pub async fn reflect(&mut self, spec: &str, test_output: &[String]) -> anyhow::Result<String> {
        let reflection_prompt = format!(
            include_str!("reflection_format.txt"),
            spec,
            test_output.join("\n")
        );
        let response = self.agent.chat(reflection_prompt).await?;
        Ok(response.choices[0].clone().message.content.unwrap())
    }
}
