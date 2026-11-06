use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LaunchStep {
    PreparingInstance,
    CheckingJavaRuntime,
    SyncingMods,
    AssemblingClasspath,
    SpawningClient,
    Complete,
}

impl LaunchStep {
    pub fn step_number(&self) -> u32 {
        match self {
            Self::PreparingInstance => 1,
            Self::CheckingJavaRuntime => 2,
            Self::SyncingMods => 3,
            Self::AssemblingClasspath => 4,
            Self::SpawningClient => 5,
            Self::Complete => 6,
        }
    }

    pub fn total_steps() -> u32 {
        6
    }

    pub fn default_message(&self) -> &'static str {
        match self {
            Self::PreparingInstance => "Preparing server instance directories...",
            Self::CheckingJavaRuntime => "Checking isolated Java runtime...",
            Self::SyncingMods => "Downloading and linking mods via CAS...",
            Self::AssemblingClasspath => "Synchronizing libraries and assembling classpath...",
            Self::SpawningClient => "Spawning Minecraft client process...",
            Self::Complete => "Minecraft process started successfully!",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub step: LaunchStep,
    pub step_number: u32,
    pub total_steps: u32,
    pub message: String,
    pub percentage: f32,
}

impl ProgressPayload {
    pub fn from_step(step: LaunchStep, custom_message: Option<&str>) -> Self {
        let step_number = step.step_number();
        let total_steps = LaunchStep::total_steps();
        let percentage = (step_number as f32 / total_steps as f32) * 100.0;
        let message = custom_message
            .unwrap_or_else(|| step.default_message())
            .to_string();

        Self {
            step,
            step_number,
            total_steps,
            message,
            percentage,
        }
    }
}

pub trait ProgressReporter: Send + Sync {
    fn report(&self, step: LaunchStep, custom_message: Option<&str>);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgressReporter;

impl ProgressReporter for NoopProgressReporter {
    fn report(&self, _step: LaunchStep, _custom_message: Option<&str>) {}
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CliProgressReporter;

impl ProgressReporter for CliProgressReporter {
    fn report(&self, step: LaunchStep, custom_message: Option<&str>) {
        let payload = ProgressPayload::from_step(step, custom_message);
        println!(
            "[szz-core] [{}/{}] {}",
            payload.step_number,
            payload.total_steps,
            payload.message
        );
    }
}
