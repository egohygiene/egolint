// Package revive provides source used to validate Revive integration.
package revive

import (
	"fmt"
	"strings"
)

// GreetingService creates personalized greeting messages.
type GreetingService struct {
	prefix string
}

// NewGreetingService creates a greeting service with a normalized prefix.
func NewGreetingService(prefix string) GreetingService {
	return GreetingService{
		prefix: strings.TrimSpace(prefix),
	}
}

// CreateGreeting creates a greeting for the supplied name.
func (service GreetingService) CreateGreeting(name string) string {
	normalizedName := strings.TrimSpace(name)
	if normalizedName == "" {
		return service.prefix
	}

	return fmt.Sprintf("%s, %s!", service.prefix, normalizedName)
}
