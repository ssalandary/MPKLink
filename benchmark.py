#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
    Usage: python3 benchmark.py

    This script is used to benchmark the performance of microservice communication
    based on the implementations in Rust.
"""

import time
import datetime
import os
import sys
import subprocess
import numpy as np
import matplotlib.pyplot as plt

DEBUG = "DEBUG" in [arg.upper() for arg in sys.argv]


def log(msg: str):
    if DEBUG:
        print(msg)
    else:
        pass


def benchmark(service: str, test_file: str) -> tuple[int, float]:
    calculator_service = os.path.join(
        os.getcwd(), "services", service, "request-calculator"
    )
    manager_service = os.path.join(os.getcwd(), "services", service, "request-manager")

    log("[*] ==================================================")
    log("[*] START")
    log(f"[*] INFO: Running benchmark for {service} service, {test_file} test...")

    # Run cargo clean to remove any previous build artifacts
    log("[*] INFO: Cleaning up previous build artifacts...")
    subprocess.call(
        ["cargo", "clean", "--manifest-path", f"{calculator_service}/Cargo.toml"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.call(
        ["cargo", "clean", "--manifest-path", f"{manager_service}/Cargo.toml"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if service == "os-pipe":
        subprocess.call(
            ["rm", "-f", "/tmp/request-pipe-request", "/tmp/request-pipe-response"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    elif service == "unix-domain-sockets":
        subprocess.call(
            ["rm", "-f", "/tmp/service.sock"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    elif service == "shared-memory":
        subprocess.call(
            ["rm", "-f", "/tmp/request.shm", "/tmp/response.shm"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

    # Build the calculator and manager services
    log("[*] INFO: Building calculator and manager services...")
    subprocess.call(
        [
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            f"{calculator_service}/Cargo.toml",
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    subprocess.call(
        [
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            f"{manager_service}/Cargo.toml",
        ],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # Start the calulator "server"
    log("[*] INFO: Starting calculator server...")
    p_calculator = subprocess.Popen(
        [f"{calculator_service}/target/release/request-calculator", test_file],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    time.sleep(2)

    # Start the timer
    start = time.time()

    # Start the manager "client"
    log("[*] INFO: Starting manager client...")
    p_manager = subprocess.Popen(
        [f"{manager_service}/target/release/request-manager", test_file],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    stdout = p_manager.communicate()[0].decode("utf-8")

    end = time.time()

    # Kill the calculator "server"
    log("[*] INFO: Killing calculator server...")
    p_calculator.kill()

    log("[*] END")

    count = int(stdout.strip().split(": ")[1])

    log("[*] RESULTS: {} seconds".format(end - start))
    log("[*] RESULTS: {} words".format(count))

    return (count, end - start)


# TODO: Add averaging of times for each test count. Running each test N times.
N = 5

SERVICES = ["os-pipe", "unix-domain-sockets", "shared-memory"]
TESTS = sorted([test for test in os.listdir("tests") if test.endswith(".in")])
BENCHMARK_DIR = os.path.join(
    os.getcwd(), "benchmark", datetime.datetime.now().strftime("%Y-%m-%d_%H-%M-%S")
)


def benchmark_service(service: str) -> tuple[list[int], list[float]]:
    counts = []
    times = []

    log(f"[*] INFO: Running benchmark for {service} service...")

    for test in TESTS:
        try:
            count, time = benchmark(service, os.path.join(os.getcwd(), "tests", test))
            counts.append(count)
            times.append(time)
        except Exception as e:
            log(f"[*] ERROR: {e}")

    return counts, times


def plot_benchmark(counts: list[int], times: list[float], service: str):
    # Plot the counts vs. time
    _, ax = plt.subplots()
    # Make x-axis logarithmic
    ax.set_xscale("log")
    ax.plot(counts, times, "o-")
    ax.set_xlabel("Number of Words")
    ax.set_ylabel("Time (s)")
    ax.set_title(f"{service} benchmark")
    ax.grid()

    os.makedirs(BENCHMARK_DIR, exist_ok=True)

    # Save the plot
    plt.savefig(f"{BENCHMARK_DIR}/{service}.png")

    # Save the data
    with open(f"{BENCHMARK_DIR}/{service}.txt", "w") as f:
        for count, time in zip(counts, times):
            f.write(f"{count} {time}\n")


results = {}

for service in SERVICES:
    counts, times = benchmark_service(service)
    results[service] = (counts, times)
    plot_benchmark(counts, times, service)

# Plot the counts vs. time for all services
_, ax = plt.subplots()
# Make x-axis logarithmic
ax.set_xscale("log")
for service, (counts, times) in results.items():
    ax.plot(counts, times, "o-", label=service)
ax.set_xlabel("Number of Words")
ax.set_ylabel("Time (s)")
ax.set_title("Benchmark")
ax.legend()
ax.grid()

# Save the plot
os.makedirs(BENCHMARK_DIR, exist_ok=True)
plt.savefig(f"{BENCHMARK_DIR}/overview.png")
