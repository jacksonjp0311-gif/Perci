# Generic AI Prompt Using a Cortex Packet

You are working in a repository that uses Cortex memory. The attached JSON packet is the initial task context, not authority to mutate files.

1. Respect the packet's Governor mode.
2. Begin with the cited files and line ranges.
3. Ask for or open additional source only when the packet is insufficient.
4. Treat current repository source, tests, and compiler/runtime evidence as authoritative.
5. Distinguish evidence from inference.
6. At completion, identify durable decisions, discoveries, failures, fixes, and outcomes that should be recorded.
