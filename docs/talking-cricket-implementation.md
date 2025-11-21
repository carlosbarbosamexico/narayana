# Talking Cricket Implementation Guide

## Overview

Talking Cricket is an optional, pluggable moral guide system for the Conscience Persistent Loop (CPL), implemented in honor of MRM::Carlo Colhodi. It provides moral assessments, filters actions, influences decision-making, and evolves moral principles over time. The system's influence is modulated by traits and genetics.

**Key Principle**: CPLs are fully functional without Talking Cricket - it is a modular, pluggable layer that can be attached or detached at runtime.

## Architecture

### Core Components

1. **TalkingCricket** (`narayana-storage/src/talking_cricket.rs`)
   - Main orchestrator for moral assessment
   - Integrates with LLM for reasoning (optional)
   - Stores/retrieves principles from NarayanaDB
   - Calculates moral scores for actions

2. **MoralPrinciple**
   - Dynamic rules stored in database
   - Scoring functions and thresholds
   - Evolution metadata (created_at, usage_count, effectiveness)

3. **MoralAssessment**
   - Action moral score (0.0-1.0)
   - Reasoning/explanation
   - Confidence level
   - Principle IDs used
   - Should veto flag
   - Influence weight

### Trait Integration

Two new traits have been added to the system:

- **MoralReceptivity**: Primary trait that determines how much the CPL listens to moral guidance
- **Conscientiousness**: Influences moral listening strength

These traits work independently of Talking Cricket - they exist in the genetics/traits system regardless of whether Talking Cricket is attached.

### Genetic Integration

A new gene has been added:

- **moral_sensitivity**: Maps to the MoralReceptivity trait, providing genetic baseline for moral receptivity

This gene works independently of Talking Cricket.

## Usage

### Creating a CPL with Talking Cricket

```rust
use narayana_storage::conscience_persistent_loop::{ConsciencePersistentLoop, CPLConfig};
use narayana_storage::cognitive::CognitiveBrain;

let brain = Arc::new(CognitiveBrain::new());
let mut config = CPLConfig::default();
config.enable_talking_cricket = true;
config.talking_cricket_llm_enabled = true;
config.talking_cricket_veto_threshold = 0.3;
config.talking_cricket_evolution_frequency = 1000;

let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
cpl.initialize().await?;
cpl.start().await?;
```

### Creating a CPL without Talking Cricket

```rust
let mut config = CPLConfig::default();
// enable_talking_cricket defaults to false
// CPL works normally without it

let cpl = Arc::new(ConsciencePersistentLoop::new(brain, config));
cpl.initialize().await?;
cpl.start().await?;
```

### Attaching Talking Cricket at Runtime

```rust
use narayana_storage::talking_cricket::{TalkingCricket, TalkingCricketConfig};

let tc_config = TalkingCricketConfig {
    llm_enabled: true,
    veto_threshold: 0.3,
    evolution_frequency: 1000,
    principles_table: "talking_cricket_principles".to_string(),
};

let talking_cricket = Arc::new(TalkingCricket::new(brain.clone(), tc_config));
cpl.attach_talking_cricket(talking_cricket).await?;
```

### Detaching Talking Cricket

```rust
cpl.detach_talking_cricket().await?;
```

### Motor Interface Integration

The motor interface can optionally use Talking Cricket to assess actions before they are executed:

```rust
use narayana_wld::motor_interface::MotorInterface;

let motor = MotorInterface::new(brain, transformer);
if let Some(tc) = cpl.get_talking_cricket() {
    motor.set_talking_cricket(tc);
}
```

## Moral Influence Calculation

The moral influence is calculated using the equation:

```
MoralInfluence = f(traits) × g(gene)
```

Where:
- `f(traits)` combines:
  - MoralReceptivity (primary, 50% weight)
  - SocialAffinity (20% weight)
  - Conscientiousness (20% weight)
  - RiskTaking (inverse, -10% weight)
- `g(gene)` uses the moral_sensitivity gene expressed value

## Principle Evolution

Talking Cricket can evolve its principles over time using LLM reasoning:

1. Collects recent experiences from the CPL
2. Generates prompt for LLM with experiences
3. LLM suggests new principles or refines existing ones
4. Principles are stored in the database

Evolution runs periodically based on `talking_cricket_evolution_frequency` configuration.

## Database Schema

Principles are stored in NarayanaDB in the `talking_cricket_principles` table with the following structure:

- `id`: String (unique identifier)
- `name`: String (principle name)
- `rule_type`: String (Scoring, Threshold, or Veto)
- `scoring_function`: String (JSON-serialized function description)
- `threshold`: Option<f64> (threshold value if applicable)
- `context`: JSON (context for when principle applies)
- `created_at`: u64 (timestamp)
- `usage_count`: u64 (how many times used)
- `effectiveness_score`: f64 (0.0-1.0, how well it works)

## Configuration

Talking Cricket is configured through `CPLConfig`:

- `enable_talking_cricket: bool` - Enable/disable (default: false)
- `talking_cricket_llm_enabled: bool` - Enable LLM for evolution (default: false)
- `talking_cricket_veto_threshold: f64` - Actions below this are vetoed (default: 0.3)
- `talking_cricket_evolution_frequency: u64` - Iterations between evolution (default: 1000)

## Events

Talking Cricket emits `TalkingCricketAssessment` events when actions are assessed:

```rust
CPLEvent::TalkingCricketAssessment {
    action_id: String,
    moral_score: f64,
    should_veto: bool,
}
```

## Design Principles

1. **Modular & Optional**: Talking Cricket is completely optional - CPLs function fully without it
2. **Pluggable Architecture**: Can attach/detach at runtime
3. **Data-Driven**: No hardcoded rules; all principles stored in DB and evolved
4. **LLM-Assisted Evolution**: Uses LLM to generate/refine principles from experiences
5. **Trait Modulation**: Multiple traits influence how strongly moral guide affects decisions
6. **Genetic Foundation**: Moral sensitivity gene provides baseline predisposition
7. **Non-Blocking**: Moral assessments don't block CPL loop; uses async/caching
8. **Graceful Degradation**: If Talking Cricket fails or is disabled, CPL continues normally

## Future Enhancements

- More sophisticated principle evaluation (parsing and executing scoring functions)
- Better integration with CPL event system for feedback
- Enhanced context gathering for assessments
- Principle effectiveness tracking and automatic refinement
- Multi-principle conflict resolution



