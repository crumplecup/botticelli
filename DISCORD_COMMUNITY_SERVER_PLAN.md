# Discord Community Server Plan for Botticelli Users

## Overview

This document outlines a plan to create a Discord server dedicated to Botticelli users, using Botticelli itself to generate and manage the server's content, structure, and community resources.

## Vision

Create a vibrant Discord community that:

1. **Serves Users**: Provides help, tutorials, and support for Botticelli users
2. **Demonstrates Capabilities**: Showcases what Botticelli can do through self-hosted examples
3. **Builds Community**: Connects developers, AI enthusiasts, and Rust programmers
4. **Generates Content**: Uses Botticelli narratives to create documentation, guides, and resources
5. **Meta-Example**: The server itself becomes a case study of using Botticelli for community building

## Server Structure

### Core Channels

#### 🏠 Welcome & Information
- **#welcome** - Auto-generated greeting and server overview (Botticelli narrative)
- **#rules** - Community guidelines (Botticelli-generated, human-reviewed)
- **#announcements** - Release notes, updates, breaking changes
- **#faq** - Frequently asked questions (narrative-generated, curated)

#### 📚 Documentation & Learning
- **#getting-started** - Quick start guides for new users
- **#tutorials** - Step-by-step walkthroughs (narrative-generated)
- **#examples** - Real-world use cases and narrative templates
- **#best-practices** - Tips and patterns from the community

#### 💬 Discussion & Support
- **#general** - General discussion about Botticelli
- **#help** - User support and troubleshooting
- **#feature-requests** - Ideas for new features
- **#bug-reports** - Issue tracking and discussion

#### 🔧 Development
- **#contributors** - For people contributing to Botticelli
- **#pull-requests** - PR discussions and reviews
- **#architecture** - Design discussions
- **#server-separation** - Discussions about external server implementations

#### 🎨 Showcase
- **#projects** - Users share their Botticelli projects
- **#narratives** - Share interesting narrative TOML files
- **#generated-content** - Cool things created with Botticelli
- **#integrations** - Custom integrations and extensions

#### 🤖 Bot Interaction
- **#bot-commands** - Interact with the Botticelli Discord bot
- **#narrative-requests** - Request the bot to run narratives
- **#scheduled-content** - Auto-posted content from narratives

## Content Generation Strategy

### Phase 1: Server Setup and Basic Content

#### 1.1 Generate Server Description and Rules

**Narrative**: `narratives/discord/server_rules.toml`

```toml
[narrative]
name = "discord_server_rules"
description = "Generate community guidelines for Botticelli Discord server"

[toc]
order = ["generate_rules"]

[acts.generate_rules]
model = "gemini-2.0-flash-exp"
max_tokens = 2000
temperature = 0.7

[[acts.generate_rules.input]]
type = "text"
content = """
Create comprehensive community guidelines for the Botticelli Discord server.

Botticelli is a Rust library and CLI for executing multi-act LLM narratives with 
support for Gemini and other providers. The community values:
- Respectful, inclusive communication
- Helping newcomers learn Rust and LLMs
- Constructive feedback on code and ideas
- Sharing knowledge and examples
- Open-source collaboration

Generate rules covering:
1. General conduct and respect
2. Technical discussion guidelines
3. Code sharing and support expectations
4. Self-promotion and spam policies
5. Moderation and enforcement

Format as Discord-friendly markdown with emojis.
"""
```

#### 1.2 Generate Welcome Message

**Narrative**: `narratives/discord/welcome_message.toml`

```toml
[narrative]
name = "discord_welcome"
description = "Generate welcome message for new Discord members"

[toc]
order = ["generate_welcome"]

[acts.generate_welcome]
model = "gemini-2.0-flash-exp"
max_tokens = 1500

[[acts.generate_welcome.input]]
type = "text"
content = """
Create an engaging welcome message for new members of the Botticelli Discord server.

Include:
1. Warm greeting
2. Brief explanation of what Botticelli is
3. Navigation guide to important channels
4. Encouragement to introduce themselves
5. Links to getting started resources
6. Reminder to read the rules

Use Discord markdown formatting and friendly emojis.
Keep it concise and welcoming.
"""
```

#### 1.3 Generate FAQ

**Narrative**: `narratives/discord/faq_generation.toml`

```toml
[narrative]
name = "discord_faq"
description = "Generate FAQ for Botticelli Discord server"

[toc]
order = ["research_common_questions", "generate_faq"]

[acts.research_common_questions]
model = "gemini-2.0-flash-exp"
max_tokens = 2000

[[acts.research_common_questions.input]]
type = "text"
content = """
Based on the Botticelli README and documentation, identify the 15 most common 
questions new users might ask. Consider:
- Installation and setup
- Basic usage
- Feature availability
- Comparison to other tools
- Troubleshooting
- Best practices
"""

[acts.generate_faq]
model = "gemini-2.0-flash-exp"
max_tokens = 3000

[[acts.generate_faq.input]]
type = "text"
content = """
Create a comprehensive FAQ document with clear, concise answers to these questions:
{{research_common_questions}}

For each question:
1. State the question clearly
2. Provide a direct answer
3. Include code examples where relevant
4. Link to relevant documentation
5. Add troubleshooting tips if applicable

Format for Discord with collapsible sections using spoiler tags.
"""
```

### Phase 2: Tutorial and Guide Generation

#### 2.1 Generate Tutorial Series

**Narrative**: `narratives/discord/tutorial_series.toml`

```toml
[narrative]
name = "discord_tutorial_series"
description = "Generate tutorial series for Botticelli Discord"

[toc]
order = ["plan_tutorial_series", "write_tutorial_1"]

[acts.plan_tutorial_series]
model = "gemini-2.0-flash-exp"
max_tokens = 2000

[[acts.plan_tutorial_series.input]]
type = "text"
content = """
Design a 5-part tutorial series for Botticelli, progressing from beginner to advanced:

Tutorial 1: Hello World Narrative
Tutorial 2: Multi-Act Workflows
Tutorial 3: Multimodal Inputs (images, audio)
Tutorial 4: Database Persistence
Tutorial 5: Advanced Patterns (loops, conditionals)

For each tutorial, outline:
- Learning objectives
- Prerequisites
- Key concepts
- Hands-on exercise
- Expected outcomes
"""

[acts.write_tutorial_1]
model = "gemini-2.0-flash-exp"
max_tokens = 3000

[[acts.write_tutorial_1.input]]
type = "text"
content = """
Write Tutorial 1: Hello World Narrative

Based on this outline:
{{plan_tutorial_series}}

Include:
1. Introduction and goals
2. Step-by-step instructions with code
3. Expected output
4. Troubleshooting common issues
5. Next steps

Format for Discord with code blocks and clear sections.
"""

# Repeat for tutorials 2-5 by adding more acts to [toc] order
```

#### 2.2 Generate Use Case Examples

**Narrative**: `narratives/discord/use_case_examples.toml`

```toml
[narrative]
name = "discord_use_case_examples"
description = "Generate use case examples for Discord"

[toc]
order = ["identify_use_cases", "write_example_narratives"]

[acts.identify_use_cases]
model = "gemini-2.0-flash-exp"

[[acts.identify_use_cases.input]]
type = "text"
content = """
Identify 10 practical use cases for Botticelli across different domains:
- Content creation
- Data analysis
- Code generation
- Document processing
- Research assistance
- Customer support
- Education
- Creative writing
- Business automation
- Developer tools

For each use case, describe the problem and how Botticelli solves it.
"""

[acts.write_example_narratives]
model = "gemini-2.0-flash-exp"
max_tokens = 4000

[[acts.write_example_narratives.input]]
type = "text"
content = """
Choose 3 of the most compelling use cases from:
{{identify_use_cases}}

For each use case, write:
1. Problem description
2. Complete narrative TOML file
3. Example inputs
4. Expected outputs
5. Explanation of how it works
6. Variations and extensions

Make the examples copy-paste ready.
"""
```

### Phase 3: Community Engagement Content

#### 3.1 Weekly Discussion Topics

**Narrative**: `narratives/discord/weekly_topics.toml`

```toml
[narrative]
name = "discord_weekly_topics"
description = "Generate weekly discussion topics for community engagement"

[toc]
order = ["generate_weekly_topics"]

[acts.generate_weekly_topics]
model = "gemini-2.0-flash-exp"

[[acts.generate_weekly_topics.input]]
type = "text"
content = """
Generate 12 engaging discussion topics for weekly community conversations, such as:
- "Show and Tell: Your Coolest Narrative"
- "LLM Ethics: Responsible AI Usage"
- "Performance Optimization Tips"
- "Multi-Model Strategies"
- "Rust Best Practices in Botticelli"

For each topic:
1. Catchy title
2. Opening questions
3. Discussion prompts
4. Related resources
5. Example narratives to try

Format as Discord posts with engaging emojis and formatting.
"""
```

#### 3.2 Highlight Reel Content

**Narrative**: `narratives/discord/community_highlights.toml`

```toml
[narrative]
name = "discord_community_highlights"
description = "Generate weekly community highlight posts"

[toc]
order = ["create_highlight_template"]

[acts.create_highlight_template]
model = "gemini-2.0-flash-exp"

[[acts.create_highlight_template.input]]
type = "text"
content = """
Create a template for weekly "Community Highlights" posts that feature:
- New members welcome
- Cool projects from #projects channel
- Helpful answers from #help
- Merged pull requests
- Interesting discussions

Make it celebratory and encouraging, formatted for Discord with emojis and mentions.
"""
```

### Phase 4: Automation and Bot Integration

#### 4.1 Discord Bot for Narrative Execution

Create a Botticelli Discord bot that can:

1. **Run Narratives on Command**
   ```
   /botticelli run tutorial-1
   /botticelli explain "multi-act workflows"
   /botticelli help "How do I use images?"
   ```

2. **Scheduled Content**
   - Daily tips in #tutorials
   - Weekly discussion topics in #general
   - Monthly community highlights

3. **Interactive Support**
   - Answer FAQs automatically
   - Generate example code on request
   - Provide debugging assistance

**Implementation**: Use `botticelli_social` Discord client with narrative execution

```rust
use botticelli_social::DiscordClient;
use botticelli_narrative::NarrativeExecutor;

async fn handle_slash_command(
    command: &str,
    args: &[String],
    channel_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let narrative_path = format!("narratives/discord/{}.toml", command);
    let executor = NarrativeExecutor::new(narrative_path)?;
    
    let result = executor.execute().await?;
    
    // Post result to Discord
    discord_client
        .send_message(channel_id, result.output)
        .await?;
    
    Ok(())
}
```

#### 4.2 Content Curation Pipeline

**Workflow**:
1. Generate content with Botticelli narratives
2. Store in database with status="draft"
3. Human review and approval
4. Auto-post to Discord on schedule
5. Track engagement and iterate

**Narrative**: `narratives/discord/content_pipeline.toml`

```toml
[narrative]
name = "discord_content_review"
description = "Review and approve Discord content"

[toc]
order = ["generate_content", "review_checklist"]

[acts]
generate_content = "Generate Discord content (details depend on specific use case)"

[acts.review_checklist]
model = "gemini-2.0-flash-exp"

[[acts.review_checklist.input]]
type = "text"
content = """
Review this Discord content for:
{{generate_content}}

Checklist:
- [ ] Tone is friendly and inclusive
- [ ] Technical accuracy
- [ ] Appropriate length for Discord
- [ ] Good markdown formatting
- [ ] No sensitive information
- [ ] Links work correctly
- [ ] Code examples are tested

Provide specific feedback for any issues found.
"""
```

## Metrics and Success Criteria

### Quantitative Metrics

1. **Member Growth**: Track server growth over time
2. **Engagement Rate**: Messages per day, active members ratio
3. **Support Effectiveness**: Time to first response, resolution rate
4. **Content Performance**: Views, reactions, discussion threads
5. **Bot Usage**: Commands per day, narrative execution success rate

### Qualitative Metrics

1. **User Feedback**: Sentiment analysis of feedback messages
2. **Onboarding Success**: New user retention and progression
3. **Community Health**: Positive interactions, constructive discussions
4. **Content Quality**: Usefulness ratings from users
5. **Brand Perception**: How users describe Botticelli to others

## Implementation Roadmap

### Month 1: Foundation
- [ ] Create Discord server with basic channel structure
- [ ] Generate and post rules, welcome message, FAQ
- [ ] Set up roles and permissions
- [ ] Create initial tutorial content
- [ ] Invite beta testers and early adopters

### Month 2: Content Library
- [ ] Generate complete tutorial series
- [ ] Create use case examples library
- [ ] Write troubleshooting guides
- [ ] Develop narrative template collection
- [ ] Launch community showcase channels

### Month 3: Automation
- [ ] Deploy Discord bot for narrative execution
- [ ] Implement scheduled content posting
- [ ] Set up content curation pipeline
- [ ] Create interactive help system
- [ ] Start weekly discussion topics

### Month 4: Growth & Engagement
- [ ] Launch community events (hackathons, challenges)
- [ ] Create contributor recognition system
- [ ] Implement feedback loops for content
- [ ] Expand bot capabilities
- [ ] Analyze metrics and iterate

## Technical Architecture

### Content Storage

```
narratives/
├── discord/
│   ├── server/
│   │   ├── rules.toml
│   │   ├── welcome.toml
│   │   └── faq.toml
│   ├── tutorials/
│   │   ├── tutorial_01_hello_world.toml
│   │   ├── tutorial_02_multi_act.toml
│   │   └── ...
│   ├── examples/
│   │   ├── content_creation.toml
│   │   ├── data_analysis.toml
│   │   └── ...
│   ├── engagement/
│   │   ├── weekly_topics.toml
│   │   ├── community_highlights.toml
│   │   └── tips_daily.toml
│   └── automation/
│       ├── help_responses.toml
│       ├── code_examples.toml
│       └── debug_assistance.toml
```

### Database Schema Extensions

```sql
-- Track Discord content generation
CREATE TABLE discord_content_generations (
    id SERIAL PRIMARY KEY,
    content_type VARCHAR(50) NOT NULL,  -- 'tutorial', 'faq', 'announcement'
    narrative_name VARCHAR(255) NOT NULL,
    generated_at TIMESTAMP NOT NULL,
    reviewed BOOLEAN DEFAULT FALSE,
    approved BOOLEAN DEFAULT FALSE,
    posted BOOLEAN DEFAULT FALSE,
    posted_at TIMESTAMP,
    channel_id BIGINT,
    message_id BIGINT,
    content TEXT NOT NULL,
    metadata JSONB
);

-- Track bot interactions
CREATE TABLE discord_bot_interactions (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    command VARCHAR(100) NOT NULL,
    arguments JSONB,
    narrative_executed VARCHAR(255),
    execution_id INTEGER REFERENCES narrative_executions(id),
    success BOOLEAN NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT,
    created_at TIMESTAMP NOT NULL
);

-- Track engagement metrics
CREATE TABLE discord_engagement_metrics (
    id SERIAL PRIMARY KEY,
    date DATE NOT NULL,
    total_members INTEGER,
    active_members INTEGER,
    messages_posted INTEGER,
    bot_commands INTEGER,
    narratives_executed INTEGER,
    avg_response_time_ms INTEGER,
    new_members INTEGER,
    UNIQUE(date)
);
```

### Bot Command System

```rust
// crates/botticelli_social/src/discord/bot.rs

pub struct BotticelliBot {
    client: DiscordClient,
    executor: NarrativeExecutor,
    db: DatabaseConnection,
}

impl BotticelliBot {
    pub async fn handle_command(
        &self,
        command: SlashCommand,
    ) -> Result<(), DiscordError> {
        match command.name.as_str() {
            "run" => self.run_narrative(command).await?,
            "help" => self.provide_help(command).await?,
            "explain" => self.explain_concept(command).await?,
            "example" => self.generate_example(command).await?,
            _ => self.unknown_command(command).await?,
        }
        Ok(())
    }
    
    async fn run_narrative(
        &self,
        command: SlashCommand,
    ) -> Result<(), DiscordError> {
        let narrative_name = command.get_string("narrative")?;
        let path = format!("narratives/discord/{}.toml", narrative_name);
        
        // Execute narrative
        let result = self.executor.execute_file(&path).await?;
        
        // Store in database
        self.db.save_interaction(&command, &result).await?;
        
        // Send response
        self.client.respond(&command, result.output).await?;
        
        Ok(())
    }
}
```

## Content Quality Guidelines

### Review Checklist for Generated Content

Before posting generated content to Discord:

1. **Technical Accuracy**
   - [ ] Code examples compile and run
   - [ ] API usage is correct for current version
   - [ ] Links point to valid resources
   - [ ] Commands produce expected results

2. **Tone and Style**
   - [ ] Friendly and welcoming
   - [ ] Inclusive language
   - [ ] Appropriate emoji usage
   - [ ] Discord markdown formatting

3. **Completeness**
   - [ ] All promised sections included
   - [ ] Examples are comprehensive
   - [ ] Prerequisites are listed
   - [ ] Next steps are clear

4. **Community Fit**
   - [ ] Aligns with server values
   - [ ] Appropriate for target audience
   - [ ] Encourages engagement
   - [ ] Fosters learning

### Iteration Process

1. **Generate** → Initial content from narrative
2. **Review** → Human checks quality and accuracy
3. **Refine** → Adjust narrative parameters based on feedback
4. **Approve** → Mark content ready for posting
5. **Post** → Publish to Discord
6. **Monitor** → Track engagement and reactions
7. **Learn** → Update narrative templates based on performance

## Risk Mitigation

### Content Quality Risks

**Risk**: AI-generated content may be inaccurate or inappropriate

**Mitigation**:
- All content goes through human review before posting
- Maintain approved content library
- Use conservative temperature settings (0.7)
- Test code examples before posting
- Regular audits of posted content

### Community Management Risks

**Risk**: Server becomes spam/low-quality discussions

**Mitigation**:
- Clear rules and active moderation
- Verification for new members
- Channel-specific guidelines
- Rate limiting on bot commands
- Report system for violations

### Technical Risks

**Risk**: Bot downtime or errors in narrative execution

**Mitigation**:
- Graceful error handling and user-friendly messages
- Fallback to manual posting if bot fails
- Monitoring and alerting for bot health
- Rate limiting to prevent abuse
- Backup content ready for posting

## Success Stories and Use Cases

Document how the server demonstrates Botticelli's value:

1. **"We Built Our Own Community"** - Meta example of using Botticelli to build itself
2. **"From Zero to Tutorial Series"** - How narratives generated all documentation
3. **"Bot That Writes Bots"** - Discord bot that teaches users to use Botticelli
4. **"Community-Driven Narratives"** - User-submitted narrative templates
5. **"Multi-Model Learning"** - Comparing different LLM outputs for same prompts

## Future Enhancements

### Advanced Bot Features
- Narrative marketplace (share and rate narratives)
- Interactive tutorials with step-by-step execution
- Code review assistance using narratives
- Project showcases with auto-generated documentation
- Community narrative challenges

### Integration Expansion
- GitHub integration for issue tracking
- Blog post auto-generation from discussions
- Video tutorial script generation
- Podcast episode planning
- Newsletter content curation

### Analytics Dashboard
- Real-time engagement metrics
- Content performance heatmaps
- Member growth trends
- Bot usage analytics
- Sentiment analysis of discussions

## Conclusion

By using Botticelli to build its own community server, we create:

1. **Living Documentation** - Server is an active example of capabilities
2. **Continuous Improvement** - Narratives improve based on real usage
3. **Community Engagement** - Members see the power of the tool they're learning
4. **Case Study** - Demonstrates practical application of LLM automation
5. **Feedback Loop** - User needs directly inform narrative development

The Discord server becomes both a resource for users and a showcase of what's possible with multi-act LLM narratives. Every piece of content, every tutorial, every help response demonstrates the power and flexibility of Botticelli.

## Getting Started

To implement this plan:

1. **Fork the Planning Doc**: Customize for your specific goals
2. **Set Up Discord Server**: Create with basic channel structure
3. **Generate Initial Content**: Start with welcome, rules, FAQ narratives
4. **Invite Beta Users**: Get early feedback on structure and content
5. **Iterate**: Refine narratives based on real user interactions
6. **Launch Bot**: Deploy automated features once content is solid
7. **Measure & Improve**: Track metrics and continuously enhance

**Remember**: The server itself is a narrative - it evolves with each interaction, each piece of generated content, each member contribution. Let Botticelli help you build the community that supports Botticelli users.
