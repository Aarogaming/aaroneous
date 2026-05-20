import React from "react";
import { BoxSelect, Database, Send } from "lucide-react";

type AgentFactoryTabProps = {
  agentForm: { name: string; domain: string; gguf_path: string };
  setAgentForm: (form: { name: string; domain: string; gguf_path: string }) => void;
  showAgentForm: boolean;
  setShowAgentForm: (value: boolean) => void;
  externalModels: { name: string; path: string; size_bytes: number; source: string }[];
  factoryStatus: string;
  handleCreateAgent: () => Promise<void>;
  handleImportDissect: (modelPath: string) => Promise<void>;
  handleForgeSovereigns: (modelPath: string) => Promise<void>;
};

function AgentFactoryTab({
  agentForm,
  setAgentForm,
  showAgentForm,
  setShowAgentForm,
  externalModels,
  factoryStatus,
  handleCreateAgent,
  handleImportDissect,
  handleForgeSovereigns,
}: AgentFactoryTabProps) {
  return (
    <div className="tab-body">
      <div
        className="tab-header"
        style={{
          padding: "0 0 24px 0",
          marginBottom: "24px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
        }}
      >
        <div>
          <h2>Synth DNA Forge</h2>
          <p>Discover GGUF models, dissect neural structures, and forge new Sovereigns.</p>
        </div>
        {factoryStatus && (
          <div
            style={{
              color: "var(--accent-solid)",
              fontSize: "14px",
              fontWeight: 600,
            }}
          >
            {factoryStatus}
          </div>
        )}
      </div>

      {showAgentForm ? (
        <div className="form-group">
          <h3>
            <BoxSelect size={18} /> Spawn Custom Agent
          </h3>
          <input
            type="text"
            className="input-field"
            placeholder="Agent Name (e.g. Architect)"
            value={agentForm.name}
            onChange={(e) =>
              setAgentForm({ ...agentForm, name: e.target.value })
            }
          />
          <input
            type="text"
            className="input-field"
            placeholder="Domain Expertise (e.g. coding)"
            value={agentForm.domain}
            onChange={(e) =>
              setAgentForm({ ...agentForm, domain: e.target.value })
            }
          />
          <input
            type="text"
            className="input-field"
            placeholder="GGUF File Path (Optional)"
            value={agentForm.gguf_path}
            onChange={(e) =>
              setAgentForm({ ...agentForm, gguf_path: e.target.value })
            }
          />
          <div style={{ display: "flex", gap: "12px" }}>
            <button className="btn btn-primary" onClick={handleCreateAgent}>
              Deploy
            </button>
            <button
              className="btn btn-secondary"
              onClick={() => setShowAgentForm(false)}
            >
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button
          className="btn btn-primary"
          style={{ marginBottom: "24px" }}
          onClick={() => setShowAgentForm(true)}
        >
          <BoxSelect size={16} /> Custom Deployment
        </button>
      )}

      <div className="form-group">
        <h3>
          <Database size={18} /> HuggingFace Fast Sync
        </h3>
        <div style={{ display: "flex", gap: "12px" }}>
          <button
            className="btn btn-secondary"
            onClick={() =>
              handleImportDissect("hf://Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF")
            }
          >
            Qwen 1.5B Coder
          </button>
          <button
            className="btn btn-secondary"
            onClick={() =>
              handleImportDissect("hf://Qwen/Qwen2.5-7B-Instruct-GGUF")
            }
          >
            Qwen 7B Core
          </button>
        </div>
        <div className="input-box" style={{ marginTop: "16px" }}>
          <input type="text" id="hf-input" placeholder="hf://Repo/Name" />
          <button
            className="send-btn"
            onClick={() => {
              const val = (document.getElementById("hf-input") as HTMLInputElement).value;
              if (val) handleImportDissect(val);
            }}
          >
            <Send size={16} />
          </button>
        </div>
      </div>

      <h3>Local Model Discovery (LM Studio / Ollama)</h3>
      <div className="grid-layout" style={{ marginTop: "16px" }}>
        {externalModels.length === 0 ? (
          <div className="empty-state">No local models detected.</div>
        ) : (
          externalModels.map((model, idx) => (
            <div key={idx} className="card">
              <div className="card-header">
                <h3 className="card-title">
                  <Database size={16} /> {model.name}
                </h3>
                <span className="badge external">{model.source}</span>
              </div>
              <div className="card-meta">{model.path}</div>
              <p className="card-desc">
                Size: {(model.size_bytes / 1073741824).toFixed(2)} GB
              </p>
              <div className="card-actions">
                <button
                  className="btn btn-secondary"
                  onClick={() => handleImportDissect(model.path)}
                >
                  Dissect
                </button>
                <button
                  className="btn btn-primary"
                  onClick={() => handleForgeSovereigns(model.path)}
                >
                  Forge
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default AgentFactoryTab;