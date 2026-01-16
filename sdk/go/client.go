// Package cage provides a Go SDK for CAGE (Contained AI-Generated Code Execution)
package cage

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"time"
)

// Client is the CAGE REST API client
type Client struct {
	BaseURL    string
	APIKey     string
	HTTPClient *http.Client
}

// NewClient creates a new CAGE client
func NewClient(baseURL, apiKey string) *Client {
	if baseURL == "" {
		baseURL = "http://127.0.0.1:8080"
	}
	if apiKey == "" {
		apiKey = "dev_user"
	}

	return &Client{
		BaseURL: baseURL,
		APIKey:  apiKey,
		HTTPClient: &http.Client{
			Timeout: 60 * time.Second,
		},
	}
}

// ExecuteRequest represents a code execution request
type ExecuteRequest struct {
	Code           string            `json:"code"`
	Language       string            `json:"language,omitempty"`
	TimeoutSeconds int               `json:"timeout_seconds,omitempty"`
	Persistent     bool              `json:"persistent,omitempty"`
	Env            map[string]string `json:"env,omitempty"`
}

// ExecuteResponse represents a code execution response
type ExecuteResponse struct {
	ExecutionID   string         `json:"execution_id"`
	Status        string         `json:"status"`
	Stdout        string         `json:"stdout"`
	Stderr        string         `json:"stderr"`
	ExitCode      *int           `json:"exit_code"`
	DurationMS    int64          `json:"duration_ms"`
	FilesCreated  []string       `json:"files_created,omitempty"`
	ResourceUsage *ResourceUsage `json:"resource_usage,omitempty"`
}

// ResourceUsage represents container resource usage
type ResourceUsage struct {
	CPUPercent float64 `json:"cpu_percent"`
	MemoryMB   float64 `json:"memory_mb"`
	DiskMB     float64 `json:"disk_mb"`
	PIDs       int     `json:"pids"`
}

// HealthResponse represents server health status
type HealthResponse struct {
	Status         string `json:"status"`
	Version        string `json:"version"`
	UptimeSeconds  int64  `json:"uptime_seconds"`
	ActiveSessions int    `json:"active_sessions"`
	PodmanVersion  string `json:"podman_version,omitempty"`
}

// FileInfo represents a file in the workspace
type FileInfo struct {
	Name        string    `json:"name"`
	Path        string    `json:"path"`
	Type        string    `json:"type"`
	SizeBytes   int64     `json:"size_bytes"`
	ModifiedAt  time.Time `json:"modified_at"`
	Permissions string    `json:"permissions,omitempty"`
}

// Execute executes code in a sandbox
func (c *Client) Execute(req *ExecuteRequest) (*ExecuteResponse, error) {
	if req.Language == "" {
		req.Language = "python"
	}
	if req.TimeoutSeconds == 0 {
		req.TimeoutSeconds = 30
	}

	body, err := json.Marshal(req)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	httpReq, err := http.NewRequest("POST", c.BaseURL+"/api/v1/execute", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Authorization", "ApiKey "+c.APIKey)

	resp, err := c.HTTPClient.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("execution failed (status %d): %s", resp.StatusCode, string(bodyBytes))
	}

	var result ExecuteResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// UploadFile uploads a file to the workspace
func (c *Client) UploadFile(localPath, targetPath string) error {
	file, err := os.Open(localPath)
	if err != nil {
		return fmt.Errorf("failed to open file: %w", err)
	}
	defer file.Close()

	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)

	part, err := writer.CreateFormFile("file", filepath.Base(localPath))
	if err != nil {
		return fmt.Errorf("failed to create form file: %w", err)
	}

	if _, err := io.Copy(part, file); err != nil {
		return fmt.Errorf("failed to copy file: %w", err)
	}

	if err := writer.WriteField("path", targetPath); err != nil {
		return fmt.Errorf("failed to write path field: %w", err)
	}

	if err := writer.Close(); err != nil {
		return fmt.Errorf("failed to close writer: %w", err)
	}

	req, err := http.NewRequest("POST", c.BaseURL+"/api/v1/files", body)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", writer.FormDataContentType())
	req.Header.Set("Authorization", "ApiKey "+c.APIKey)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("upload failed (status %d): %s", resp.StatusCode, string(bodyBytes))
	}

	return nil
}

// DownloadFile downloads a file from the workspace
func (c *Client) DownloadFile(filePath, outputPath string) error {
	req, err := http.NewRequest("GET", c.BaseURL+"/api/v1/files/"+filePath, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Authorization", "ApiKey "+c.APIKey)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("download failed (status %d): %s", resp.StatusCode, string(bodyBytes))
	}

	out, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("failed to create output file: %w", err)
	}
	defer out.Close()

	if _, err := io.Copy(out, resp.Body); err != nil {
		return fmt.Errorf("failed to write file: %w", err)
	}

	return nil
}

// ListFiles lists files in the workspace
func (c *Client) ListFiles(path string, recursive bool) ([]FileInfo, error) {
	params := url.Values{}
	params.Set("path", path)
	if recursive {
		params.Set("recursive", "true")
	}

	req, err := http.NewRequest("GET", c.BaseURL+"/api/v1/files?"+params.Encode(), nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Authorization", "ApiKey "+c.APIKey)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		bodyBytes, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("list files failed (status %d): %s", resp.StatusCode, string(bodyBytes))
	}

	var result struct {
		Files []FileInfo `json:"files"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return result.Files, nil
}

// Health gets server health status
func (c *Client) Health() (*HealthResponse, error) {
	resp, err := http.Get(c.BaseURL + "/health")
	if err != nil {
		return nil, fmt.Errorf("health check failed: %w", err)
	}
	defer resp.Body.Close()

	var health HealthResponse
	if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &health, nil
}
