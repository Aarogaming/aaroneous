import React from "react";
import { Box, Layers, Settings } from "lucide-react";

type SABArsenalTabProps = {
  arsenal: { name: string; domain: string; kind: string }[];
  setAgentForm: (form: { name: string; domain: string; gguf_path: string }) => void;
  setShowAgentForm: (value: boolean) => void;
  setActiveTab: (tab: string) => void;
};

function SABArsenalTab({ arsenal, setAgentForm, setShowAgentForm, setActiveTab }: SABArsenalTabProps) {
  return (
    <div className="tab-body">
      <div className="tab-header" style={{ padding: "0 0 24px 0", marginBottom: "24px" }}>
        <h2>Sovereign Agent Bundles</h2>
        <p>Dynamically compiled WASM plugins and Native DLLs actively loaded in memory.</p>
      </div>
      <div className="grid-layout">
        {arsenal.map((spec, idx) => (
          <div key={idx} className="card">
            <div className="card-header">
              <h3 className="card-title"><Box size={18} /> {spec.name}</h3>
              <span className={`badge ${spec.kind === "core" ? "native" : "wasm"}`}>{spec.kind}</span>
            </div>
            <p className="card-desc">{spec.domain}</p>
            <div className="card-actions">
              <button 
                className="btn btn-secondary" 
                onClick={() => {
                  setAgentForm({ name: spec.name, domain: spec.domain, gguf_path: "" });
                  setShowAgentForm(true);
                  setActiveTab("agent_factory");
                }}
              >
                <Settings size={14} /> Configure
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default SABArsenalTab;