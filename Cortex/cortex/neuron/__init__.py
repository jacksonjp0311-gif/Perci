"""Sparse neural interlink integrated with Cortex's single memory substrate."""

from .compiler import compile_interlink, neural_graph_state
from .engine import activate_interlink
from .models import NeuralActivationPacket, NeuralActivationRecord, NeuralNode, NeuralSynapse

__all__ = [
    "NeuralActivationPacket",
    "NeuralActivationRecord",
    "NeuralNode",
    "NeuralSynapse",
    "activate_interlink",
    "compile_interlink",
    "neural_graph_state",
]
