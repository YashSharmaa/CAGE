package main

import (
	"fmt"
	"log"
	"strings"

	cage "github.com/cage-project/cage/sdk/go"
)

func main() {
	fmt.Println("Testing CAGE Go SDK...")
	fmt.Println(strings.Repeat("=", 60))

	client := cage.NewClient("http://127.0.0.1:8080", "dev_gosdk")

	// Test 1: Execute Python
	fmt.Println("\n[1/3] Testing Execute()...")
	result, err := client.Execute(&cage.ExecuteRequest{
		Code:     "print('Go SDK Test: 789')",
		Language: "python",
	})
	if err != nil {
		log.Fatalf("Execute failed: %v", err)
	}
	fmt.Printf("✓ Execute passed (status: %s, output: %s)\n", result.Status, result.Stdout)

	// Test 2: Health
	fmt.Println("\n[2/3] Testing Health()...")
	health, err := client.Health()
	if err != nil {
		log.Fatalf("Health failed: %v", err)
	}
	fmt.Printf("✓ Health passed (status: %s, version: %s)\n", health.Status, health.Version)

	// Test 3: List files
	fmt.Println("\n[3/3] Testing ListFiles()...")
	files, err := client.ListFiles("/", false)
	if err != nil {
		log.Fatalf("ListFiles failed: %v", err)
	}
	fmt.Printf("✓ List files passed (%d files)\n", len(files))

	fmt.Println("\n" + strings.Repeat("=", 60))
	fmt.Println("✅ ALL GO SDK TESTS PASSED!")
	fmt.Println(strings.Repeat("=", 60))
}
