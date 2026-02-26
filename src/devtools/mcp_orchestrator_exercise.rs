// Dev-only MCP orchestrator exercise. Enable with --features devtools.

use crate::mcp::{
    extract_entities, format_operation, AgentConfig, AgentRequest, AgentResponse, AgentRole,
    EditOperation, Entity, HostingMode, IntentType, IssueSeverity, McpMessage, McpOrchestrator,
    McpServer, Provider, TaskStatus, TokenUsage, UserIntent,
};

pub fn exercise_mcp_orchestrator() {
    let mut orchestrator = McpOrchestrator::new();

    // Exercise AgentRole variants with their methods.
    let roles = [
        AgentRole::Voice,
        AgentRole::Orchestrator,
        AgentRole::Coder,
        AgentRole::Auditor,
    ];
    for role in &roles {
        let _n = role.name();
        let _i = role.icon();
        let _d = role.description();
    }

    // Provider checks.
    let providers = [
        Provider::Xai,
        Provider::Together,
        Provider::OpenAI,
        Provider::Ollama,
        Provider::Custom,
    ];
    for p in &providers {
        let _compat = p.is_openai_compatible();
        let _local = p.is_local();
        let _name = p.name();
    }

    // TaskStatus and IssueSeverity icons.
    let statuses = [
        TaskStatus::Pending,
        TaskStatus::InProgress,
        TaskStatus::Review,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Blocked,
    ];
    for s in &statuses {
        let _icon = s.icon();
    }
    let severities = [
        IssueSeverity::Info,
        IssueSeverity::Warning,
        IssueSeverity::Error,
        IssueSeverity::Critical,
    ];
    for sv in &severities {
        let _icon = sv.icon();
    }

    // AgentConfig::huggingface with builder methods.
    let hf_config = AgentConfig::huggingface(
        AgentRole::Coder,
        "https://api.hf.space",
        "codellama/CodeLlama-34b",
    )
    .with_url("https://custom.endpoint/v1")
    .with_key("hf_test_key")
    .with_model("custom-model");
    orchestrator.configure_agent(hf_config);

    // Set API key.
    orchestrator.set_api_key(AgentRole::Voice, "test_key".into());

    // Read McpOrchestrator public fields.
    let _tid = orchestrator.next_task_id;

    // Start session and process input.
    orchestrator.start_session();
    let _msgs = orchestrator.process_input("explain this code");

    // Build a request.
    let msgs: Vec<McpMessage> = Vec::new();
    if let Some(req) = orchestrator.build_request(AgentRole::Coder, &msgs) {
        let _req: &AgentRequest = &req;
        let _model = &req.model;
        let _msgs = &req.messages;
        let _max = req.max_tokens;
        let _temp = req.temperature;
    }

    // Extract entities.
    let entities: Vec<Entity> = extract_entities("fix bug in src/main.rs line 42");
    for e in &entities {
        let _et = &e.entity_type;
        let _ev = &e.value;
        let _es = e.start;
        let _ee = e.end;
    }

    // Format operations.
    let ops = [
        EditOperation::Create,
        EditOperation::Replace,
        EditOperation::Insert,
        EditOperation::Delete,
        EditOperation::Append,
    ];
    for op in &ops {
        let _f = format_operation(op);
    }

    // Exercise all IntentType variants via UserIntent and read fields.
    let intents = [
        IntentType::Create,
        IntentType::Fix,
        IntentType::Refactor,
        IntentType::Explain,
        IntentType::Test,
        IntentType::Document,
        IntentType::General,
    ];
    for it in &intents {
        let ui = UserIntent {
            summary: format!("do {:?}", it),
            intent_type: it.clone(),
            entities: entities.clone(),
            confidence: 0.95,
        };
        let _s = &ui.summary;
        let _t = &ui.intent_type;
        let _e = &ui.entities;
        let _c = ui.confidence;
    }

    // Mock response with TokenUsage.
    let resp: AgentResponse = McpOrchestrator::mock_response("test response");
    let _content = &resp.content;
    let _finish = &resp.finish_reason;
    let usage: &TokenUsage = &resp.usage;
    let _pt = usage.prompt_tokens;
    let _ct = usage.completion_tokens;
    let _tt = usage.total_tokens;

    // Session summary.
    let _summary = orchestrator.session_summary();

    // Configure hosting mode.
    orchestrator.configure_hosting(HostingMode::Cloud);
    orchestrator.set_all_api_keys("test_key_all");

    // Git integration methods.
    let _branches = orchestrator.git_branches();
    let _log = orchestrator.git_log(5);
    let _diff = orchestrator.git_diff(false);
    let _diff_file = orchestrator.git_diff_file("src/main.rs", true);
    let _blame = orchestrator.git_blame("src/main.rs");
    let _commit = orchestrator.git_show_commit("abc123");
    let _ = orchestrator.git_stage(&["src/main.rs"]);
    let _ = orchestrator.git_unstage(&["src/main.rs"]);
    let _cid = orchestrator.git_queue_commit("test commit", None);
    let _bid = orchestrator.git_queue_branch("feature/test", None);
    let _pending = orchestrator.git_pending_ops();
    let _ = orchestrator.git_approve(0);
    let _oid = orchestrator.git_queue_operation("tag", "v1.0");

    // File system methods.
    let _ = orchestrator.fs_read_file("test.txt");
    let _ = orchestrator.fs_read_lines("test.txt", 0, 10);
    let _ = orchestrator.fs_list_dir(".");
    let _ = orchestrator.fs_list_recursive(".", 2);
    let _ = orchestrator.fs_search("*.rs");
    let _ = orchestrator.fs_grep("TODO", Some("*.rs"));
    let _ = orchestrator.fs_queue_create("new.txt", "content", "Create new file");
    let _ = orchestrator.fs_queue_update("old.txt", "updated", "Update file");
    let _ = orchestrator.fs_queue_delete("temp.txt", "Remove temp file");
    let _pc = orchestrator.fs_pending_changes();
    let _ = orchestrator.fs_approve(0);
    let _ = orchestrator.fs_approve_all();
    let _ = orchestrator.fs_reject(0);
    orchestrator.fs_reject_all();
    let _ = orchestrator.fs_history();
    let _ = orchestrator.fs_generate_diff("old", "new", "test.txt");
    let _ = orchestrator.fs_queue_rename("a.txt", "b.txt", "Rename file");
    let _ = orchestrator.fs_queue_mkdir("new_dir", "Create directory");
    let _ = orchestrator.fs_copy_file("src.txt", "dst.txt");
    let _ = orchestrator.fs_get_cached("test.txt");

    // McpServer lifecycle with builder methods.
    let server = McpServer::new(9090);
    let server = server.with_agent_url(AgentRole::Voice, "http://localhost:8001");
    let server = server.with_agent_key(AgentRole::Voice, "key123");
    let mut server = server.with_agent_model(AgentRole::Coder, "gpt-4");
    server.start();
    let _resp = server.handle_request(r#"{\"method\":\"ping\"}"#);
    let _status = server.status();
    server.stop();
}
