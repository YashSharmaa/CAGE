// CAGE Admin TUI - Terminal User Interface for CAGE Orchestrator administration
//
// This tool provides real-time monitoring and management of CAGE sandbox sessions.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"sort"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/help"
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/table"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// Styles
var (
	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#FAFAFA")).
			Background(lipgloss.Color("#7D56F4")).
			Padding(0, 1)

	statusBarStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFFDF5")).
			Background(lipgloss.Color("#353533"))

	helpStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#626262"))

	errorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FF0000"))

	successStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#00FF00"))

	warningStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#FFFF00"))
)

// API Response types
type HealthResponse struct {
	Status         string  `json:"status"`
	Version        string  `json:"version"`
	UptimeSeconds  int64   `json:"uptime_seconds"`
	ActiveSessions int     `json:"active_sessions"`
	PodmanVersion  *string `json:"podman_version"`
}

type SessionSummary struct {
	UserID         string    `json:"user_id"`
	ContainerID    *string   `json:"container_id"`
	Status         string    `json:"status"`
	CreatedAt      time.Time `json:"created_at"`
	LastActivity   time.Time `json:"last_activity"`
	CPUPercent     float64   `json:"cpu_percent"`
	MemoryMB       float64   `json:"memory_mb"`
	ExecutionCount int64     `json:"execution_count"`
	ErrorCount     int64     `json:"error_count"`
	Warnings       []string  `json:"warnings"`
}

type SessionListResponse struct {
	Sessions []SessionSummary `json:"sessions"`
	Total    int64            `json:"total"`
}

type SystemStats struct {
	UptimeSeconds           int64   `json:"uptime_seconds"`
	ActiveSessions          int     `json:"active_sessions"`
	TotalExecutions         int64   `json:"total_executions"`
	ExecutionsLastHour      int     `json:"executions_last_hour"`
	AverageExecutionTimeMs  float64 `json:"average_execution_time_ms"`
	TotalErrors             int64   `json:"total_errors"`
	ErrorsLastHour          int     `json:"errors_last_hour"`
	SecurityEventsLastHour  int     `json:"security_events_last_hour"`
}

type ReplayEntry struct {
	ExecutionID string    `json:"execution_id"`
	UserID      string    `json:"user_id"`
	Timestamp   time.Time `json:"timestamp"`
	Language    string    `json:"language"`
	Status      string    `json:"status"`
}

type UserEntry struct {
	UserID    string `json:"user_id"`
	Enabled   bool   `json:"enabled"`
	Languages int    `json:"allowed_languages_count"`
}

// Key bindings
type keyMap struct {
	Up       key.Binding
	Down     key.Binding
	Refresh  key.Binding
	Kill     key.Binding
	Details  key.Binding
	Back     key.Binding
	Replays  key.Binding
	Users    key.Binding
	Quit     key.Binding
	Help     key.Binding
}

func (k keyMap) ShortHelp() []key.Binding {
	return []key.Binding{k.Refresh, k.Details, k.Replays, k.Users, k.Quit}
}

func (k keyMap) FullHelp() [][]key.Binding {
	return [][]key.Binding{
		{k.Up, k.Down, k.Refresh},
		{k.Kill, k.Details, k.Back},
		{k.Help, k.Quit},
	}
}

var keys = keyMap{
	Up: key.NewBinding(
		key.WithKeys("up", "k"),
		key.WithHelp("↑/k", "up"),
	),
	Down: key.NewBinding(
		key.WithKeys("down", "j"),
		key.WithHelp("↓/j", "down"),
	),
	Refresh: key.NewBinding(
		key.WithKeys("r"),
		key.WithHelp("r", "refresh"),
	),
	Kill: key.NewBinding(
		key.WithKeys("x"),
		key.WithHelp("x", "terminate"),
	),
	Details: key.NewBinding(
		key.WithKeys("enter"),
		key.WithHelp("enter", "details"),
	),
	Back: key.NewBinding(
		key.WithKeys("esc"),
		key.WithHelp("esc", "back"),
	),
	Replays: key.NewBinding(
		key.WithKeys("p"),
		key.WithHelp("p", "replays"),
	),
	Users: key.NewBinding(
		key.WithKeys("u"),
		key.WithHelp("u", "users"),
	),
	Quit: key.NewBinding(
		key.WithKeys("q", "ctrl+c"),
		key.WithHelp("q", "quit"),
	),
	Help: key.NewBinding(
		key.WithKeys("?"),
		key.WithHelp("?", "help"),
	),
}

// Messages
type tickMsg time.Time
type healthMsg HealthResponse
type sessionsMsg []SessionSummary
type statsMsg SystemStats
type errorMsg struct{ err error }
type terminateMsg struct{ success bool; userID string }

// Model
type model struct {
	apiURL      string
	token       string

	health      *HealthResponse
	sessions    []SessionSummary
	stats       *SystemStats

	table       table.Model
	spinner     spinner.Model
	help        help.Model
	keys        keyMap

	loading     bool
	err         error
	lastUpdate  time.Time

	width       int
	height      int

	view        string // "main", "details", "replays", "users"
	selected    *SessionSummary
	replays     []ReplayEntry
	users       []UserEntry
}

func initialModel(apiURL, token string) model {
	columns := []table.Column{
		{Title: "User", Width: 15},
		{Title: "Status", Width: 10},
		{Title: "CPU %", Width: 8},
		{Title: "Mem MB", Width: 8},
		{Title: "Execs", Width: 8},
		{Title: "Errors", Width: 8},
		{Title: "Last Activity", Width: 20},
	}

	t := table.New(
		table.WithColumns(columns),
		table.WithFocused(true),
		table.WithHeight(10),
	)

	s := table.DefaultStyles()
	s.Header = s.Header.
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(lipgloss.Color("240")).
		BorderBottom(true).
		Bold(true)
	s.Selected = s.Selected.
		Foreground(lipgloss.Color("229")).
		Background(lipgloss.Color("57")).
		Bold(false)
	t.SetStyles(s)

	sp := spinner.New()
	sp.Spinner = spinner.Dot
	sp.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

	return model{
		apiURL:  apiURL,
		token:   token,
		table:   t,
		spinner: sp,
		help:    help.New(),
		keys:    keys,
		loading: true,
		view:    "main",
	}
}

func (m model) Init() tea.Cmd {
	return tea.Batch(
		m.spinner.Tick,
		m.fetchHealth(),
		m.fetchSessions(),
		m.fetchStats(),
		tea.Every(5*time.Second, func(t time.Time) tea.Msg {
			return tickMsg(t)
		}),
	)
}

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, m.keys.Quit):
			return m, tea.Quit
		case key.Matches(msg, m.keys.Refresh):
			m.loading = true
			return m, tea.Batch(
				m.fetchHealth(),
				m.fetchSessions(),
				m.fetchStats(),
			)
		case key.Matches(msg, m.keys.Kill):
			if m.view == "main" && len(m.sessions) > 0 {
				idx := m.table.Cursor()
				if idx < len(m.sessions) {
					return m, m.terminateSession(m.sessions[idx].UserID)
				}
			}
		case key.Matches(msg, m.keys.Details):
			if m.view == "main" && len(m.sessions) > 0 {
				idx := m.table.Cursor()
				if idx < len(m.sessions) {
					m.selected = &m.sessions[idx]
					m.view = "details"
				}
			}
		case key.Matches(msg, m.keys.Back):
			if m.view != "main" {
				m.view = "main"
				m.selected = nil
			}
		case key.Matches(msg, m.keys.Replays):
			if m.view == "main" {
				m.view = "replays"
			}
		case key.Matches(msg, m.keys.Users):
			if m.view == "main" {
				m.view = "users"
			}
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.table.SetHeight(msg.Height - 15)

	case tickMsg:
		return m, tea.Batch(
			m.fetchSessions(),
			m.fetchStats(),
			tea.Every(5*time.Second, func(t time.Time) tea.Msg {
				return tickMsg(t)
			}),
		)

	case healthMsg:
		h := HealthResponse(msg)
		m.health = &h
		m.loading = false

	case sessionsMsg:
		m.sessions = msg
		m.updateTable()
		m.loading = false
		m.lastUpdate = time.Now()

	case statsMsg:
		s := SystemStats(msg)
		m.stats = &s

	case errorMsg:
		m.err = msg.err
		m.loading = false

	case terminateMsg:
		if msg.success {
			// Refresh sessions
			return m, m.fetchSessions()
		}

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)
	}

	// Update table
	var cmd tea.Cmd
	m.table, cmd = m.table.Update(msg)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

func (m *model) updateTable() {
	rows := make([]table.Row, len(m.sessions))
	for i, s := range m.sessions {
		statusStyle := lipgloss.NewStyle()
		switch s.Status {
		case "running":
			statusStyle = successStyle
		case "stopped":
			statusStyle = warningStyle
		case "error":
			statusStyle = errorStyle
		}

		rows[i] = table.Row{
			s.UserID,
			statusStyle.Render(s.Status),
			fmt.Sprintf("%.1f", s.CPUPercent),
			fmt.Sprintf("%.0f", s.MemoryMB),
			fmt.Sprintf("%d", s.ExecutionCount),
			fmt.Sprintf("%d", s.ErrorCount),
			s.LastActivity.Format("2006-01-02 15:04:05"),
		}
	}
	m.table.SetRows(rows)
}

func (m model) View() string {
	switch m.view {
	case "details":
		if m.selected != nil {
			return m.detailsView()
		}
	case "replays":
		return m.replaysView()
	case "users":
		return m.usersView()
	}
	return m.mainView()
}

func (m model) mainView() string {
	var b strings.Builder

	// Title
	title := titleStyle.Render(" CAGE Admin Console ")
	b.WriteString(title)
	b.WriteString("\n\n")

	// Health status
	if m.health != nil {
		statusIcon := "●"
		statusColor := "#00FF00"
		if m.health.Status == "degraded" {
			statusColor = "#FFFF00"
		} else if m.health.Status == "unhealthy" {
			statusColor = "#FF0000"
		}

		status := lipgloss.NewStyle().Foreground(lipgloss.Color(statusColor)).Render(statusIcon)
		b.WriteString(fmt.Sprintf("Status: %s %s  |  Version: %s  |  Uptime: %s  |  Sessions: %d\n",
			status,
			m.health.Status,
			m.health.Version,
			formatDuration(m.health.UptimeSeconds),
			m.health.ActiveSessions,
		))
	}

	// Stats
	if m.stats != nil {
		b.WriteString(fmt.Sprintf("Executions: %d total, %d/hr  |  Errors: %d total, %d/hr  |  Security Events: %d/hr\n",
			m.stats.TotalExecutions,
			m.stats.ExecutionsLastHour,
			m.stats.TotalErrors,
			m.stats.ErrorsLastHour,
			m.stats.SecurityEventsLastHour,
		))
	}

	b.WriteString("\n")

	// Loading indicator or table
	if m.loading && len(m.sessions) == 0 {
		b.WriteString(m.spinner.View() + " Loading sessions...")
	} else {
		b.WriteString(m.table.View())
	}

	b.WriteString("\n\n")

	// Last update time
	if !m.lastUpdate.IsZero() {
		b.WriteString(helpStyle.Render(fmt.Sprintf("Last updated: %s", m.lastUpdate.Format("15:04:05"))))
		b.WriteString("\n")
	}

	// Error
	if m.err != nil {
		b.WriteString(errorStyle.Render(fmt.Sprintf("Error: %v", m.err)))
		b.WriteString("\n")
	}

	// Help
	b.WriteString("\n")
	b.WriteString(m.help.View(m.keys))

	return b.String()
}

func (m model) detailsView() string {
	var b strings.Builder

	title := titleStyle.Render(fmt.Sprintf(" Session: %s ", m.selected.UserID))
	b.WriteString(title)
	b.WriteString("\n\n")

	s := m.selected

	// Session details
	details := []struct {
		label string
		value string
	}{
		{"Status", s.Status},
		{"Container ID", stringOrNA(s.ContainerID)},
		{"Created", s.CreatedAt.Format("2006-01-02 15:04:05")},
		{"Last Activity", s.LastActivity.Format("2006-01-02 15:04:05")},
		{"CPU Usage", fmt.Sprintf("%.1f%%", s.CPUPercent)},
		{"Memory Usage", fmt.Sprintf("%.0f MB", s.MemoryMB)},
		{"Total Executions", fmt.Sprintf("%d", s.ExecutionCount)},
		{"Total Errors", fmt.Sprintf("%d", s.ErrorCount)},
	}

	for _, d := range details {
		b.WriteString(fmt.Sprintf("  %-18s %s\n", d.label+":", d.value))
	}

	// Warnings
	if len(s.Warnings) > 0 {
		b.WriteString("\n")
		b.WriteString(warningStyle.Render("Warnings:"))
		b.WriteString("\n")
		for _, w := range s.Warnings {
			b.WriteString(fmt.Sprintf("  - %s\n", w))
		}
	}

	b.WriteString("\n")
	b.WriteString(helpStyle.Render("Press ESC to go back, x to terminate"))

	return b.String()
}

func (m model) replaysView() string {
	var b strings.Builder

	title := titleStyle.Render(" Execution Replays ")
	b.WriteString(title)
	b.WriteString("\n\n")

	b.WriteString("Recent executions stored for replay:\n\n")
	b.WriteString(helpStyle.Render("Note: Replay data loaded from /api/v1/replays"))
	b.WriteString("\n\n")
	b.WriteString("(Replay list fetching not yet implemented in TUI)")
	b.WriteString("\n\n")
	b.WriteString(helpStyle.Render("Press ESC to go back, 'u' for users, 'p' for replays"))

	return b.String()
}

func (m model) usersView() string {
	var b strings.Builder

	title := titleStyle.Render(" User Management ")
	b.WriteString(title)
	b.WriteString("\n\n")

	b.WriteString("Configured users:\n\n")
	b.WriteString(helpStyle.Render("Note: User data loaded from /api/v1/admin/users"))
	b.WriteString("\n\n")
	b.WriteString("(User list fetching not yet implemented in TUI)")
	b.WriteString("\n\n")
	b.WriteString(helpStyle.Render("Press ESC to go back, 'u' for users, 'p' for replays"))

	return b.String()
}

// API calls
func (m model) fetchHealth() tea.Cmd {
	return func() tea.Msg {
		resp, err := m.doRequest("GET", "/health", nil)
		if err != nil {
			return errorMsg{err}
		}
		defer resp.Body.Close()

		var health HealthResponse
		if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
			return errorMsg{err}
		}
		return healthMsg(health)
	}
}

func (m model) fetchSessions() tea.Cmd {
	return func() tea.Msg {
		resp, err := m.doRequest("GET", "/api/v1/admin/sessions", nil)
		if err != nil {
			return errorMsg{err}
		}
		defer resp.Body.Close()

		var result SessionListResponse
		if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
			return errorMsg{err}
		}

		// Sort by last activity
		sort.Slice(result.Sessions, func(i, j int) bool {
			return result.Sessions[i].LastActivity.After(result.Sessions[j].LastActivity)
		})

		return sessionsMsg(result.Sessions)
	}
}

func (m model) fetchStats() tea.Cmd {
	return func() tea.Msg {
		resp, err := m.doRequest("GET", "/api/v1/admin/stats", nil)
		if err != nil {
			return errorMsg{err}
		}
		defer resp.Body.Close()

		var stats SystemStats
		if err := json.NewDecoder(resp.Body).Decode(&stats); err != nil {
			return errorMsg{err}
		}
		return statsMsg(stats)
	}
}

func (m model) terminateSession(userID string) tea.Cmd {
	return func() tea.Msg {
		resp, err := m.doRequest("DELETE", "/api/v1/admin/sessions/"+userID, nil)
		if err != nil {
			return errorMsg{err}
		}
		resp.Body.Close()

		return terminateMsg{success: resp.StatusCode == 204, userID: userID}
	}
}

func (m model) doRequest(method, path string, body io.Reader) (*http.Response, error) {
	req, err := http.NewRequest(method, m.apiURL+path, body)
	if err != nil {
		return nil, err
	}

	if m.token != "" {
		req.Header.Set("Authorization", "ApiKey "+m.token)
	}
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 10 * time.Second}
	return client.Do(req)
}

// Helpers
func formatDuration(seconds int64) string {
	d := time.Duration(seconds) * time.Second
	if d < time.Hour {
		return fmt.Sprintf("%dm", int(d.Minutes()))
	}
	if d < 24*time.Hour {
		return fmt.Sprintf("%dh %dm", int(d.Hours()), int(d.Minutes())%60)
	}
	days := int(d.Hours()) / 24
	hours := int(d.Hours()) % 24
	return fmt.Sprintf("%dd %dh", days, hours)
}

func stringOrNA(s *string) string {
	if s == nil || *s == "" {
		return "N/A"
	}
	return *s
}

func main() {
	apiURL := flag.String("api", "http://localhost:8080", "CAGE API URL")
	token := flag.String("token", "", "Admin API token")
	flag.Parse()

	// Check for env vars
	if *token == "" {
		*token = os.Getenv("CAGE_ADMIN_TOKEN")
	}
	if envAPI := os.Getenv("CAGE_API_URL"); envAPI != "" {
		*apiURL = envAPI
	}

	p := tea.NewProgram(
		initialModel(*apiURL, *token),
		tea.WithAltScreen(),
	)

	if _, err := p.Run(); err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}
}
