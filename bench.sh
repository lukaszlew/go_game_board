#!/bin/bash

set -e

echo "=== High-Performance Rust Benchmark Setup ==="

# Preserve original user's PATH for cargo
ORIGINAL_PATH="$PATH"
ORIGINAL_HOME="$HOME"

# 1. Disable ASLR (Address Space Layout Randomization)
echo "Disabling ASLR..."
sudo sh -c 'echo 0 > /proc/sys/kernel/randomize_va_space' 2>/dev/null || echo "  Skipped (sudo failed)"

# 2. Set CPU governor to performance mode
echo "Setting CPU governor to performance..."
sudo sh -c 'for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    if [ -w "$cpu" ]; then
        echo performance > "$cpu" 2>/dev/null || true
    fi
done' 2>/dev/null || echo "  Skipped (sudo failed)"

# 3. Disable CPU frequency boost (Intel Turbo Boost / AMD Turbo Core)
echo "Disabling CPU frequency boost..."
# Intel
sudo sh -c 'if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
    echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo
fi' 2>/dev/null || true
# AMD
sudo sh -c 'if [ -f /sys/devices/system/cpu/cpufreq/boost ]; then
    echo 0 > /sys/devices/system/cpu/cpufreq/boost
fi' 2>/dev/null || true

# 4. Select CPU core for pinning (using CPU 2 to avoid system interrupts on CPU 0)
CPU_CORE=2
echo "Will pin process to CPU core $CPU_CORE"

# 5. Isolate the CPU core (make it exclusive)
echo "Isolating CPU core $CPU_CORE..."
MAX_CPU=$(($(nproc --all) - 1))
sudo sh -c "for irq in /proc/irq/*/smp_affinity_list; do
    if [ -w \"\$irq\" ]; then
        # Set affinity to all CPUs except our target
        echo '0-1,3-$MAX_CPU' > \"\$irq\" 2>/dev/null || true
    fi
done" 2>/dev/null || echo "  Skipped (sudo failed)"

# 6. Build and run with optimized settings
echo "Building and running with optimized settings..."

# Build first with custom profile
echo "Building with release-lto profile..."
cargo build --profile release-lto

# Run with CPU affinity using custom profile
echo "Running benchmark pinned to CPU $CPU_CORE..."
taskset -c $CPU_CORE cargo test --profile release-lto --test benchmark_test -- --nocapture

# Cleanup function
cleanup() {
    echo ""
    echo "Restoring system settings..."
    # Re-enable ASLR
    sudo sh -c 'echo 2 > /proc/sys/kernel/randomize_va_space' 2>/dev/null || true
    
    # Reset CPU governor to ondemand or powersave
    sudo sh -c 'for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
        if [ -w "$cpu" ]; then
            echo ondemand > "$cpu" 2>/dev/null || echo powersave > "$cpu" 2>/dev/null || true
        fi
    done' 2>/dev/null || true
    
    # Re-enable CPU boost
    sudo sh -c 'if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
        echo 0 > /sys/devices/system/cpu/intel_pstate/no_turbo
    fi' 2>/dev/null || true
    sudo sh -c 'if [ -f /sys/devices/system/cpu/cpufreq/boost ]; then
        echo 1 > /sys/devices/system/cpu/cpufreq/boost
    fi' 2>/dev/null || true
    
    echo "System settings restored."
}

# Set trap to cleanup on exit
trap cleanup EXIT