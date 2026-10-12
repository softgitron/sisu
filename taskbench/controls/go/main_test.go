package main

import (
	"strconv"
	"testing"
)

// The memory guard of idle-memory and max-tasks: the headroom never exceeds
// MemAvailable, nor the memory cgroup's limit when one is set.
func TestMemHeadroom(t *testing.T) {
	h, avail := memHeadroom(), memAvailable()
	if h <= 0 || h > avail {
		t.Fatalf("headroom %d, MemAvailable %d", h, avail)
	}
	if text, ok := cgroupFile("memory", "memory.limit_in_bytes", "memory.max"); ok {
		if limit, err := strconv.ParseInt(text, 10, 64); err == nil && limit < 1<<60 && h > limit {
			t.Fatalf("headroom %d above the cgroup limit %d", h, limit)
		}
		t.Logf("cgroup memory limit %s", text)
	}
	t.Logf("headroom %d, MemAvailable %d", h, avail)
}
