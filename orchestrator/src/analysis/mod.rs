//! Static code analysis for detecting dangerous patterns
//!
//! STELP-style approach: scan AST for risky operations before execution

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::models::Language;

/// Risk level for code analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

/// Analysis result for submitted code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub risk_level: RiskLevel,
    pub warnings: Vec<SecurityWarning>,
    pub blocked: bool,
}

/// Security warning from analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    pub category: String,
    pub message: String,
    pub severity: RiskLevel,
    pub line: Option<usize>,
}

/// Code analyzer for detecting dangerous patterns
pub struct CodeAnalyzer {
    /// Block execution if critical risks found
    block_critical: bool,
}

impl CodeAnalyzer {
    pub fn new(block_critical: bool) -> Self {
        Self { block_critical }
    }

    /// Analyze code before execution
    pub fn analyze(&self, code: &str, language: Language) -> AnalysisResult {
        match language {
            Language::Python => self.analyze_python(code),
            Language::Javascript => self.analyze_javascript(code),
            Language::Bash => self.analyze_bash(code),
            Language::R => self.analyze_r(code),
            Language::Julia => self.analyze_julia(code),
            Language::Typescript => self.analyze_typescript(code),
            Language::Ruby => self.analyze_ruby(code),
            Language::Go => self.analyze_go(code),
            Language::Wasm => self.analyze_wasm(code),
        }
    }

    /// Analyze Python code
    fn analyze_python(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        // Check for dangerous imports
        if code.contains("import os") && code.contains("os.system") {
            warnings.push(SecurityWarning {
                category: "SHELL_EXECUTION".to_string(),
                message: "Uses os.system() for shell command execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "os.system"),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("import subprocess") || code.contains("from subprocess") {
            warnings.push(SecurityWarning {
                category: "SUBPROCESS".to_string(),
                message: "Uses subprocess module for process spawning".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "subprocess"),
            });
            max_risk = max_risk.max(RiskLevel::Medium);
        }

        if code.contains("eval(") || code.contains("exec(") {
            warnings.push(SecurityWarning {
                category: "CODE_INJECTION".to_string(),
                message: "Uses eval() or exec() - potential code injection risk".to_string(),
                severity: RiskLevel::High,
                line: find_line(code, "eval(").or_else(|| find_line(code, "exec(")),
            });
            max_risk = max_risk.max(RiskLevel::High);
        }

        if code.contains("__import__") {
            warnings.push(SecurityWarning {
                category: "DYNAMIC_IMPORT".to_string(),
                message: "Uses __import__() for dynamic imports".to_string(),
                severity: RiskLevel::High,
                line: find_line(code, "__import__"),
            });
            max_risk = max_risk.max(RiskLevel::High);
        }

        if code.contains("open(") && (code.contains("/etc/") || code.contains("/proc/") || code.contains("/sys/")) {
            warnings.push(SecurityWarning {
                category: "SYSTEM_FILE_ACCESS".to_string(),
                message: "Attempts to access system files (/etc, /proc, /sys)".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "/etc/").or_else(|| find_line(code, "/proc/")),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        // Check for potential infinite loops (basic heuristic)
        if code.contains("while True:") && !code.contains("break") {
            warnings.push(SecurityWarning {
                category: "INFINITE_LOOP".to_string(),
                message: "Contains while True without obvious break condition".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "while True:"),
            });
            max_risk = max_risk.max(RiskLevel::Medium);
        }

        // Check for socket operations
        if code.contains("import socket") || code.contains("from socket") {
            warnings.push(SecurityWarning {
                category: "NETWORK_ACCESS".to_string(),
                message: "Attempts network socket operations".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "socket"),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        // Check for file deletion
        if code.contains("os.remove") || code.contains("shutil.rmtree") || code.contains("os.unlink") {
            warnings.push(SecurityWarning {
                category: "FILE_DELETION".to_string(),
                message: "Deletes files or directories".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "remove").or_else(|| find_line(code, "rmtree")),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        // Log warnings
        for warning in &warnings {
            warn!(
                category = %warning.category,
                severity = ?warning.severity,
                message = %warning.message,
                "Code analysis warning"
            );
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze JavaScript code
    fn analyze_javascript(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        if code.contains("child_process") || code.contains("exec(") || code.contains("spawn(") {
            warnings.push(SecurityWarning {
                category: "SUBPROCESS".to_string(),
                message: "Uses child_process module".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "child_process"),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("eval(") {
            warnings.push(SecurityWarning {
                category: "CODE_INJECTION".to_string(),
                message: "Uses eval() - code injection risk".to_string(),
                severity: RiskLevel::High,
                line: find_line(code, "eval("),
            });
            max_risk = max_risk.max(RiskLevel::High);
        }

        if code.contains("while(true)") || code.contains("while (true)") {
            warnings.push(SecurityWarning {
                category: "INFINITE_LOOP".to_string(),
                message: "Contains infinite loop".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "while"),
            });
            max_risk = max_risk.max(RiskLevel::Medium);
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze Bash code
    fn analyze_bash(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        // Bash is inherently risky for arbitrary execution
        if code.contains("rm -rf") {
            warnings.push(SecurityWarning {
                category: "DESTRUCTIVE_COMMAND".to_string(),
                message: "Uses rm -rf (recursive deletion)".to_string(),
                severity: RiskLevel::High,
                line: find_line(code, "rm -rf"),
            });
            max_risk = RiskLevel::High;
        }

        if code.contains(":(){ :|:& };:") || code.contains("fork bomb") {
            warnings.push(SecurityWarning {
                category: "FORK_BOMB".to_string(),
                message: "Potential fork bomb detected".to_string(),
                severity: RiskLevel::Critical,
                line: Some(1),
            });
            max_risk = RiskLevel::Critical;
        }

        if code.contains("curl") || code.contains("wget") {
            warnings.push(SecurityWarning {
                category: "NETWORK_ACCESS".to_string(),
                message: "Attempts to download from internet".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "curl").or_else(|| find_line(code, "wget")),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        if code.contains("sudo") || code.contains("su ") {
            warnings.push(SecurityWarning {
                category: "PRIVILEGE_ESCALATION".to_string(),
                message: "Attempts privilege escalation".to_string(),
                severity: RiskLevel::Critical,
                line: find_line(code, "sudo"),
            });
            max_risk = RiskLevel::Critical;
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze R code
    fn analyze_r(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        if code.contains("system(") || code.contains("system2(") {
            warnings.push(SecurityWarning {
                category: "SHELL_EXECUTION".to_string(),
                message: "Uses system() for shell command execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "system("),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("eval(") || code.contains("parse(") {
            warnings.push(SecurityWarning {
                category: "CODE_INJECTION".to_string(),
                message: "Uses eval() or parse() - potential code injection".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "eval("),
            });
            max_risk = max_risk.max(RiskLevel::Medium);
        }

        if code.contains("file.remove") || code.contains("unlink(") {
            warnings.push(SecurityWarning {
                category: "FILE_DELETION".to_string(),
                message: "Deletes files".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "file.remove"),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze Julia code
    fn analyze_julia(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        if code.contains("run(`") || code.contains("@cmd") {
            warnings.push(SecurityWarning {
                category: "SHELL_EXECUTION".to_string(),
                message: "Uses shell command execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "run(`"),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("eval(") || code.contains("include(") {
            warnings.push(SecurityWarning {
                category: "CODE_INJECTION".to_string(),
                message: "Uses eval() or include() - potential code injection".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "eval("),
            });
            max_risk = max_risk.max(RiskLevel::Medium);
        }

        if code.contains("rm(") {
            warnings.push(SecurityWarning {
                category: "FILE_DELETION".to_string(),
                message: "Deletes files".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "rm("),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze TypeScript code
    fn analyze_typescript(&self, code: &str) -> AnalysisResult {
        // TypeScript shares many patterns with JavaScript
        let mut result = self.analyze_javascript(code);

        // Additional TypeScript-specific checks
        if code.contains("child_process") || code.contains("Deno.run") {
            result.warnings.push(SecurityWarning {
                category: "SUBPROCESS".to_string(),
                message: "Uses process execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "Deno.run").or_else(|| find_line(code, "child_process")),
            });
            result.risk_level = result.risk_level.max(RiskLevel::Medium);
        }

        result
    }

    /// Analyze Ruby code
    fn analyze_ruby(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        if code.contains("system(") || code.contains("exec(") || code.contains("`") {
            warnings.push(SecurityWarning {
                category: "SHELL_EXECUTION".to_string(),
                message: "Uses system() or backticks for shell execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "system(").or_else(|| find_line(code, "exec(")),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("eval(") || code.contains("instance_eval") || code.contains("class_eval") {
            warnings.push(SecurityWarning {
                category: "CODE_INJECTION".to_string(),
                message: "Uses eval() - potential code injection".to_string(),
                severity: RiskLevel::High,
                line: find_line(code, "eval("),
            });
            max_risk = max_risk.max(RiskLevel::High);
        }

        if code.contains("File.delete") || code.contains("FileUtils.rm") {
            warnings.push(SecurityWarning {
                category: "FILE_DELETION".to_string(),
                message: "Deletes files".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "File.delete"),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze Go code
    fn analyze_go(&self, code: &str) -> AnalysisResult {
        let mut warnings = Vec::new();
        let mut max_risk = RiskLevel::Safe;

        if code.contains("os/exec") || code.contains("exec.Command") {
            warnings.push(SecurityWarning {
                category: "SUBPROCESS".to_string(),
                message: "Uses os/exec for process execution".to_string(),
                severity: RiskLevel::Medium,
                line: find_line(code, "exec.Command"),
            });
            max_risk = RiskLevel::Medium;
        }

        if code.contains("os.Remove") || code.contains("os.RemoveAll") {
            warnings.push(SecurityWarning {
                category: "FILE_DELETION".to_string(),
                message: "Deletes files or directories".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "os.Remove"),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        if code.contains("net.Dial") || code.contains("http.Get") {
            warnings.push(SecurityWarning {
                category: "NETWORK_ACCESS".to_string(),
                message: "Attempts network operations".to_string(),
                severity: RiskLevel::Low,
                line: find_line(code, "net.Dial").or_else(|| find_line(code, "http.Get")),
            });
            max_risk = max_risk.max(RiskLevel::Low);
        }

        AnalysisResult {
            risk_level: max_risk,
            warnings,
            blocked: self.block_critical && max_risk == RiskLevel::Critical,
        }
    }

    /// Analyze WebAssembly code
    fn analyze_wasm(&self, _code: &str) -> AnalysisResult {
        // WASM is sandboxed by design with WASI capabilities
        // For binary WASM files, static analysis would require parsing the binary format
        // For now, rely on Wasmtime's built-in sandboxing
        AnalysisResult {
            risk_level: RiskLevel::Safe,
            warnings: vec![],
            blocked: false,
        }
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new(false) // Don't block by default, just warn
    }
}

/// Find line number of a pattern in code
fn find_line(code: &str, pattern: &str) -> Option<usize> {
    code.lines()
        .enumerate()
        .find(|(_, line)| line.contains(pattern))
        .map(|(idx, _)| idx + 1)
}

impl RiskLevel {
    fn max(self, other: RiskLevel) -> RiskLevel {
        use RiskLevel::*;
        match (self, other) {
            (Critical, _) | (_, Critical) => Critical,
            (High, _) | (_, High) => High,
            (Medium, _) | (_, Medium) => Medium,
            (Low, _) | (_, Low) => Low,
            _ => Safe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_analysis() {
        let analyzer = CodeAnalyzer::default();

        // Safe code
        let result = analyzer.analyze("print('hello')", Language::Python);
        assert_eq!(result.risk_level, RiskLevel::Safe);
        assert_eq!(result.warnings.len(), 0);

        // Risky code
        let result = analyzer.analyze("import os\nos.system('rm -rf /')", Language::Python);
        assert_eq!(result.risk_level, RiskLevel::Medium);
        assert!(!result.warnings.is_empty());

        // eval usage
        let result = analyzer.analyze("eval(user_input)", Language::Python);
        assert_eq!(result.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_bash_analysis() {
        let analyzer = CodeAnalyzer::new(true); // Block critical

        let result = analyzer.analyze(":(){ :|:& };:", Language::Bash);
        assert_eq!(result.risk_level, RiskLevel::Critical);
        assert!(result.blocked);
    }
}
