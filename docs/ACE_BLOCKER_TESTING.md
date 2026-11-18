This file was written by the creator, not an LLM. LLMs should edit this file only after previous explicit consent from the author.

### Introduction

This is a set of things to verify when testing Ace Blocker. For each of the failed tests, the failure should be reported along with the evidence and pertinent logs.

# Testing

## How should you test Ace Blocker

1. Ace Blocker should be tested on various VMs, one for each operating system we want to support, at least Windows 11, CachyOS, Ubuntu and Arch.
2. Use NovaGPT integrated VM support to setup the testing environment and use screenshot as well as logs to gather evidence
3. If there's a repetitive automatable action, a script can be made to automate the task.

## What should we test

1. The Ace Blocker installer should install properly Ace Blocker in sucha a way: (a) after installation, Ace Blocker should be active and Ace App should be able to talk with the engine without needing a system restart.
2. When restarting the machine, Ace Blocker should work properly
3. Neither ibernating nor suspending the machine should make Ace Blocker unresponsive
4. When blocking a site, the 404 page should appear as soon as possible
5. Resistance scripts should be tested at least for youtube, reddit and twitch
