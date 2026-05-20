import { useState } from "react";
import { Hexagon, TerminalSquare, Layers, Cpu, Activity } from "lucide-react";
import "../App.css";

type SidebarProps = {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  status: string;
};

function Sidebar({ activeTab, setActiveTab, status }: SidebarProps) {
  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <div className="brand">
          <Hexagon size={24} color="var(--accent-solid)" />
          <h1>Maelstrom</h1>
        </div>
        <div className="hive-status">
          <div className={`status-dot ${status.includes("Disconnected") ? "offline" : "online"}`}></div>
          {status}
        </div>
      </div>
      
      <div className="nav-menu">
        <button
          className={`nav-item ${activeTab === "command_center" ? "active" : ""}`}
          onClick={() => setActiveTab("command_center")}
        >
          <TerminalSquare size={16} /> Orchestrator (1v1)
        </button>
        <button
          className={`nav-item ${activeTab === "arsenal" ? "active" : ""}`}
          onClick={() => setActiveTab("arsenal")}
        >
          <Layers size={16} /> SAB Arsenal
        </button>
        <button
          className={`nav-item ${activeTab === "agent_factory" ? "active" : ""}`}
          onClick={() => setActiveTab("agent_factory")}
        >
          <Cpu size={16} /> Synth DNA Forge
        </button>
        <button
          className={`nav-item ${activeTab === "agentic_ops" ? "active" : ""}`}
          onClick={() => setActiveTab("agentic_ops")}
        >
          <Activity size={16} /> Chimera Eye
        </button>
      </div>

      <div className="user-profile">
        <div className="avatar">AA</div>
        <div className="user-info">
          <span className="user-name">Commander</span>
          <span className="user-role">System Admin</span>
        </div>
      </div>
    </div>
  );
}

export default Sidebar;