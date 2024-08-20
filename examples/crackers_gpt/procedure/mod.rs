use std::io::Write;

use async_openai::config::OpenAIConfig;
use async_openai::Client;
use jingle::modeling::ModeledBlock;
use jingle::sleigh::Instruction;
use tempfile::{tempdir, NamedTempFile};
use tracing::{event, Level};

use crackers::config::specification::SpecificationConfig;
use crackers::synthesis::DecisionResult;

use crate::agents::assembly::AssemblyAgent;
use crate::agents::model::Model;
use crate::agents::reflection::ReflectionAgent;
use crate::config::CrackersGptConfig;
use crate::evaluator::{summarize_implementation_error, ExecveEvaluation, ExecveEvaluator};
use crate::procedure::AssemblyResult::{Failure, Success};
use crate::specification::AssemblyParametersBuilder;

pub struct GptProcedure {
    config: CrackersGptConfig,
    assembly_agent: AssemblyAgent<OpenAIConfig>,
    pub reflection_agent: ReflectionAgent<OpenAIConfig>,
    evaluator: ExecveEvaluator,
    max_iterations: usize,
}

pub enum AssemblyResult {
    Success(Vec<Instruction>),
    Failure(String),
}

impl GptProcedure {
    pub fn new(config: CrackersGptConfig) -> anyhow::Result<Self> {
        Ok(Self {
            assembly_agent: AssemblyAgent::new(Client::new(), Model::Gpt4o),
            reflection_agent: ReflectionAgent::new(Client::new(), Model::Gpt4o),
            evaluator: ExecveEvaluator::new(config.clone())?,
            max_iterations: 4,
            config,
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<DecisionResult<ModeledBlock>> {
        let mut assembly_program = self.assembly_agent.code(include_str!("user.txt")).await?;
        let assembly_result = self.assemble(&assembly_program)?;
        let evaluation = match assembly_result {
            Success(instructions) => match self.evaluator.eval(&instructions)? {
                ExecveEvaluation::SpecError(s) => s,
                ExecveEvaluation::ImplementationError(a) => {
                    summarize_implementation_error(&instructions, a)
                }
                ExecveEvaluation::Success(a) => return Ok(DecisionResult::AssignmentFound(a)),
            },
            Failure(s) => vec![s],
        };
        let mut reflection = self
            .reflection_agent
            .reflect(&assembly_program, evaluation.as_slice())
            .await
            .unwrap();
        self.assembly_agent.reset_for_reflection();
        let max = self.max_iterations;
        for _ in 0..max {
            let reflection_prompt = format!(
                include_str!("reflection_format.txt"),
                include_str!("user.txt"),
                assembly_program,
                evaluation.join("\n"),
                reflection
            );
            assembly_program = self.assembly_agent.code(reflection_prompt).await?;
            let assembly_result = self.assemble(&assembly_program)?;
            let evaluation = match assembly_result {
                Success(instructions) => match self.evaluator.eval(&instructions)? {
                    ExecveEvaluation::SpecError(s) => s,
                    ExecveEvaluation::ImplementationError(a) => {
                        summarize_implementation_error(&instructions, a)
                    }
                    ExecveEvaluation::Success(a) => return Ok(DecisionResult::AssignmentFound(a)),
                },
                Failure(s) => vec![s],
            };
            reflection = self
                .reflection_agent
                .reflect(&assembly_program, evaluation.as_slice())
                .await
                .unwrap();
        }
        todo!()
    }

    fn assemble(&self, assembly: &str) -> anyhow::Result<AssemblyResult> {
        let dir = tempdir()?;
        let asm = self.create_assembly_file(assembly)?;
        let params = AssemblyParametersBuilder::default()
            .target("x86_64-unknown-linux-gnu")
            .compiler("x86_64-unknown-linux-gnu-gcc")
            .host("aarch64-apple-darwin")
            .build()?;
        let obj_path = params.assemble(&dir, &asm);
        if obj_path.is_err() {
            return Ok(Failure(
                "Unable to compile program. Did you use AT&T syntax?".to_string(),
            ));
        }
        let obj_path = obj_path.unwrap();
        let spec_config = SpecificationConfig {
            path: obj_path[0].to_str().unwrap().to_string(),
            max_instructions: 10,
        };
        let spec = spec_config.get_spec(&self.config.sleigh)?;
        for x in &spec {
            event!(Level::DEBUG, "{}", x.disassembly);
        }
        Ok(Success(spec))
    }

    fn create_assembly_file(&self, asm: &str) -> anyhow::Result<NamedTempFile> {
        let mut a = tempfile::Builder::new()
            .suffix(".S")
            .prefix("spec")
            .tempfile()?;
        a.write_all(asm.as_bytes())?;
        Ok(a)
    }
}
