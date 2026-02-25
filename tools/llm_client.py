#!/usr/bin/env python3
"""
Shared LLM client for MOC generation scripts.

Provides a unified interface for generating descriptions of code,
documentation, and script files. Defaults to a local in-process
Transformers harness for stability; Ollama remains opt-in.

Backend selection via MOC_LLM_BACKEND env var:
  - "local" (default): in-process Transformers (MOC_LOCAL_MODEL_PATH)
  - "ollama": local Ollama HTTP/CLI fallback

Author: @darianrosebrook
"""

import json
import os
import re
import subprocess
import urllib.error
import urllib.request
from typing import Dict, List, Optional, Tuple


# Action verbs organized by content type
_SHARED_ACTION_VERBS = [
    "Provides", "Implements", "Defines", "Contains", "Manages",
    "Handles", "Processes", "Creates", "Generates", "Validates",
    "Parses", "Extracts", "Computes", "Tracks", "Maintains",
    "Enables", "Supports", "Integrates", "Derives", "Builds",
    "Loads", "Stores", "Serializes", "Deserializes", "Transforms",
    "Converts", "Maps", "Routes", "Dispatches", "Coordinates",
    "Orchestrates", "Configures", "Initializes", "Registers", "Binds",
]

_CODE_VERBS = [
    "Enforces", "Verifies", "Certifies", "Attests", "Witnesses",
    "Guards", "Gates", "Audits", "Records", "Logs",
    "Searches", "Explores", "Expands", "Evaluates", "Scores",
    "Ranks", "Selects", "Filters", "Prunes", "Backtracks",
]

_DOCUMENTATION_VERBS = [
    "Documents", "Describes", "Explains", "Covers", "Outlines", "Details",
]

_SCRIPT_VERBS = [
    "Runs", "Executes", "Analyzes", "Evaluates", "Migrates",
]

_VERBS_BY_TYPE: Dict[str, List[str]] = {
    "code": _CODE_VERBS + _SHARED_ACTION_VERBS + _DOCUMENTATION_VERBS + _SCRIPT_VERBS,
    "documentation": _DOCUMENTATION_VERBS + _SHARED_ACTION_VERBS + _CODE_VERBS + _SCRIPT_VERBS,
    "script": _SCRIPT_VERBS + _SHARED_ACTION_VERBS + _CODE_VERBS + _DOCUMENTATION_VERBS,
}

_SKIP_PATTERNS = [
    "thinking", "let me", "i'll", "i will", "looking at", "based on",
    "the module", "the script", "the file", "this module",
    "putting it together", "final answer", "here is", "so the",
    "maybe", "hmm", "that seems", "i think", "okay", "first",
    "let's see", "i need to", "now,", "...done thinking",
    "alright", "okay so", "let me generate", "generating",
]

_PREAMBLE_PREFIXES = [
    "The module ", "This module ", "The file ", "This file ",
    "This Python module ", "The Python module ",
    "Here is the description: ", "Description: ",
]

# Local in-process model defaults
_DEFAULT_LOCAL_MODEL_PATH = os.getenv(
    "MOC_LOCAL_MODEL_PATH",
    "/Users/darianrosebrook/Desktop/Projects/models/Olmo-3-7B-Instruct"
)
_LOCAL_MODEL = None
_LOCAL_TOKENIZER = None
_LOCAL_DEVICE = None


_ROLE_BY_CONTENT_TYPE = {
    "code": "You are a technical code analyst for the Sterling Native project.",
    "documentation": "You are a technical documentation analyst for the Sterling Native project.",
    "script": "You are a technical automation script analyst for the Sterling Native project.",
}


def _format_prompt_sections(
    path: str,
    title: Optional[str],
    category: Optional[str],
    summary: Optional[str],
    preview: Optional[str],
    context_fields: Optional[Dict[str, str]],
) -> str:
    """Build a compact prompt payload from available context sections."""
    sections: List[str] = [f"PATH: {path}"]
    if title:
        sections.append(f"TITLE: {title}")
    if category:
        sections.append(f"CATEGORY: {category}")
    if summary:
        sections.append(f"SUMMARY:\n{summary.strip()}")
    if context_fields:
        for key, value in context_fields.items():
            if value:
                sections.append(f"{key.upper()}:\n{str(value).strip()}")
    if preview:
        sections.append(f"CONTENT PREVIEW:\n{preview.strip()}")
    return "\n\n".join(sections)


def build_moc_description_prompt(
    content_type: str,
    *,
    path: str,
    title: Optional[str] = None,
    category: Optional[str] = None,
    summary: Optional[str] = None,
    preview: Optional[str] = None,
    project_context: Optional[str] = None,
    context_fields: Optional[Dict[str, str]] = None,
    max_chars: int = 500,
) -> Tuple[str, str]:
    """Build a shared prompt pair for MOC descriptions."""
    role = _ROLE_BY_CONTENT_TYPE.get(content_type, _ROLE_BY_CONTENT_TYPE["code"])
    verbs = ", ".join(get_action_verbs(content_type)[:12])
    sentence_target = "1-3 sentences" if content_type == "code" else "1-2 sentences"
    system_parts = [
        role,
        "Task: produce a concise technical description for MOC indexing.",
    ]
    if project_context:
        system_parts.append(project_context.strip())
    system_parts.append(
        "\n".join([
            "Rules:",
            f"1. Write {sentence_target} starting with an action verb ({verbs}, ...).",
            "2. Use only evidence present in the provided context and preview.",
            "3. Do not invent behavior, dependencies, or architecture not shown.",
            "4. Do not include meta-commentary, chain-of-thought, markdown, or preamble.",
            f"5. Keep description <= {max_chars} characters.",
            '6. Return exactly valid JSON: {"description":"..."}',
        ])
    )
    system_prompt = "\n\n".join(system_parts)

    prompt = (
        "Generate the description for this MOC entry using only the provided context.\n\n"
        f"{_format_prompt_sections(path, title, category, summary, preview, context_fields)}\n\n"
        'Return only JSON: {"description":"..."}'
    )
    return prompt, system_prompt


def get_action_verbs(content_type: str = "code") -> List[str]:
    """Return action verbs prioritized for *content_type*."""
    return list(_VERBS_BY_TYPE.get(content_type, _SHARED_ACTION_VERBS))


def call_llm(
    prompt: str,
    system_prompt: str,
    timeout: int = 180,
    model: str = "olmo-3:latest",
) -> Optional[str]:
    """Call configured backend and return raw output, or None on failure.

    Backend selection:
      - local (default): in-process Transformers
      - ollama: local Ollama HTTP/CLI fallback
    """
    backend = os.getenv("MOC_LLM_BACKEND", "local").strip().lower()
    if backend in {"local", "transformers", "hf", "harness"}:
        return _call_local_hf(prompt, system_prompt, timeout=timeout)

    api_response = _call_ollama_http(prompt, system_prompt, timeout=timeout, model=model)
    if api_response is not None:
        return api_response or None

    # CLI fallback
    try:
        result = subprocess.run(
            ["ollama", "run", model, "--nowordwrap"],
            input=f"{system_prompt}\n\n{prompt}",
            capture_output=True, text=True, timeout=timeout,
        )
        if result.returncode != 0:
            return None
        response = result.stdout.strip()
        return response if response else None
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
        return None


def _call_local_hf(
    prompt: str,
    system_prompt: str,
    timeout: int,
) -> Optional[str]:
    """Generate using local in-process Transformers model."""
    del timeout
    global _LOCAL_MODEL, _LOCAL_TOKENIZER, _LOCAL_DEVICE

    if _LOCAL_MODEL is None or _LOCAL_TOKENIZER is None:
        try:
            import torch
            from transformers import AutoModelForCausalLM, AutoTokenizer
        except Exception as e:
            print(f"  Local model dependencies unavailable: {e}")
            return None

        model_path = _DEFAULT_LOCAL_MODEL_PATH
        try:
            if torch.backends.mps.is_available():
                _LOCAL_DEVICE = "mps"
            elif torch.cuda.is_available():
                _LOCAL_DEVICE = "cuda"
            else:
                _LOCAL_DEVICE = "cpu"

            _LOCAL_TOKENIZER = AutoTokenizer.from_pretrained(model_path)
            _LOCAL_MODEL = AutoModelForCausalLM.from_pretrained(
                model_path,
                torch_dtype=torch.float16 if _LOCAL_DEVICE in {"mps", "cuda"} else torch.float32,
                device_map=_LOCAL_DEVICE,
                low_cpu_mem_usage=True,
            )
        except Exception as e:
            print(f"  Failed to load local model at {model_path}: {e}")
            _LOCAL_MODEL = None
            _LOCAL_TOKENIZER = None
            return None

    try:
        import torch

        try:
            full_prompt = _LOCAL_TOKENIZER.apply_chat_template(
                [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": prompt},
                ],
                tokenize=False,
                add_generation_prompt=True,
            )
        except Exception:
            full_prompt = (
                "<|im_start|>system\n"
                f"{system_prompt}<|im_end|>\n"
                "<|im_start|>user\n"
                f"{prompt}<|im_end|>\n"
                "<|im_start|>assistant\n"
            )

        inputs = _LOCAL_TOKENIZER(full_prompt, return_tensors="pt")
        inputs = {k: v.to(_LOCAL_DEVICE) for k, v in inputs.items()}
        input_len = inputs["input_ids"].shape[1]

        with torch.no_grad():
            outputs = _LOCAL_MODEL.generate(
                **inputs,
                max_new_tokens=180,
                do_sample=False,
                pad_token_id=_LOCAL_TOKENIZER.eos_token_id,
            )

        new_tokens = outputs[0][input_len:]
        text = _LOCAL_TOKENIZER.decode(new_tokens, skip_special_tokens=True).strip()
        return text if text else None
    except Exception as e:
        print(f"  Local generation error: {e}")
        return None


def _call_ollama_http(
    prompt: str,
    system_prompt: str,
    timeout: int,
    model: str,
) -> Optional[str]:
    """Call local Ollama HTTP generate API."""
    payload = {
        "model": model,
        "prompt": prompt,
        "system": system_prompt,
        "stream": False,
        "options": {"temperature": 0.2, "top_p": 0.9, "num_predict": 180},
        "think": False,
    }

    req = urllib.request.Request(
        "http://127.0.0.1:11434/api/generate",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8", errors="replace")
            data = json.loads(body)
            text = (data.get("response") or "").strip()
            if text:
                return text
            thinking = (data.get("thinking") or "").strip()
            if thinking:
                extracted = _extract_from_thinking(thinking)
                return extracted or ""
            return ""
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, json.JSONDecodeError):
        return None


def _extract_from_thinking(thinking: str) -> Optional[str]:
    """Extract a concise final answer from a thinking trace."""
    quoted = re.findall(r'[""]([^""]{12,280}[.!?])[""]', thinking)
    if quoted:
        return quoted[-1].strip()

    verbs = get_action_verbs("code")
    action = re.compile(r"^\s*(%s)\b" % "|".join(re.escape(v) for v in verbs))
    sentences = re.split(r"(?<=[.!?])\s+", thinking)
    candidates = [s.strip() for s in sentences if 20 <= len(s.strip()) <= 320]
    for sentence in reversed(candidates):
        if action.match(sentence):
            return sentence
    if candidates:
        return candidates[-1]
    return None


def _filter_meta_commentary(response: str) -> str:
    """Remove thinking / meta-commentary lines."""
    if not response:
        return response
    response = re.sub(r"<think>.*?</think>", "", response, flags=re.DOTALL | re.IGNORECASE)
    lines = response.split("\n")
    filtered: List[str] = []
    in_output_block = False
    for line in lines:
        lower_line = line.lower().strip()
        if any(p in lower_line for p in _SKIP_PATTERNS) and not in_output_block:
            continue
        if line.strip().startswith("OUTPUT:"):
            in_output_block = True
        elif line.strip() and line.strip()[0].isupper() and ":" in line[:20]:
            in_output_block = False
        filtered.append(line)
    return "\n".join(filtered).strip()


def parse_llm_response(
    response: str,
    content_type: str = "code",
) -> Optional[str]:
    """Extract a clean description from raw LLM output."""
    if not response:
        return None
    response = _filter_meta_commentary(response)
    if not response:
        return None

    action_verbs = get_action_verbs(content_type)
    lines = response.split("\n")

    # Strategy 1: consecutive lines starting with an action verb
    description_lines: List[str] = []
    collecting = False
    for line in lines:
        line_stripped = line.strip()
        if not line_stripped:
            if collecting:
                break
            continue
        starts_with_verb = any(line_stripped.startswith(v) for v in action_verbs)
        if starts_with_verb:
            collecting = True
            description_lines.append(line_stripped)
        elif collecting:
            if line_stripped[0].islower() or line_stripped.startswith("("):
                description_lines.append(line_stripped)
            else:
                break
    if description_lines:
        return " ".join(description_lines)

    # Strategy 2: JSON fallback
    try:
        if "{" in response and "}" in response:
            json_match = re.search(r"\{[^}]+\}", response, re.DOTALL)
            if json_match:
                data = json.loads(json_match.group())
                if "description" in data:
                    return data["description"]
    except (json.JSONDecodeError, KeyError):
        pass

    # Strategy 3: best-scoring paragraph
    paragraphs = response.split("\n\n")
    best_paragraph: Optional[str] = None
    best_score = 0
    for para in paragraphs:
        para = para.strip()
        if len(para) < 30 or len(para) > 800:
            continue
        score = len(para)
        if any(para.startswith(v) for v in action_verbs):
            score += 200
        if any(t in para.lower() for t in ["implements", "provides", "defines", "handles"]):
            score += 100
        if score > best_score:
            best_score = score
            best_paragraph = para
    if best_paragraph:
        return best_paragraph

    # Strategy 4: last reasonable non-commentary line
    for line in reversed(lines):
        line_stripped = line.strip()
        if line_stripped and 20 < len(line_stripped) < 500:
            lower_line = line_stripped.lower()
            if not any(p in lower_line for p in _SKIP_PATTERNS):
                return line_stripped
    return None


def _truncate_at_sentence(text: str, max_chars: int) -> str:
    """Truncate text at a sentence boundary."""
    if len(text) <= max_chars:
        return text
    sentence_ends = list(re.finditer(r"[.!?]\s+", text[:max_chars + 50]))
    if sentence_ends:
        for match in reversed(sentence_ends):
            if match.end() <= max_chars:
                return text[:match.end()].strip()
    truncated = text[:max_chars]
    last_space = truncated.rfind(" ")
    if last_space > max_chars * 0.7:
        return truncated[:last_space] + "..."
    return truncated + "..."


def clean_description(response: Optional[str], max_chars: int = 500) -> str:
    """Clean a parsed description for display."""
    if not response:
        return "No description available."
    response = response.strip()
    for prefix in _PREAMBLE_PREFIXES:
        if response.startswith(prefix):
            response = response[len(prefix):]
            if response:
                response = response[0].upper() + response[1:]
    response = response.strip("\"'")
    while response.endswith(".."):
        response = response[:-1]
    return _truncate_at_sentence(response, max_chars)


def generate_description(
    prompt: str,
    system_prompt: str,
    content_type: str = "code",
    timeout: int = 180,
    model: str = "olmo-3:latest",
    max_chars: int = 500,
) -> Optional[str]:
    """End-to-end helper: call LLM -> parse -> clean."""
    raw = call_llm(prompt, system_prompt, timeout=timeout, model=model)
    if raw is None:
        return None
    parsed = parse_llm_response(raw, content_type=content_type)
    if parsed is None:
        return None
    cleaned = clean_description(parsed, max_chars=max_chars)
    if cleaned == "No description available.":
        return None
    return cleaned
