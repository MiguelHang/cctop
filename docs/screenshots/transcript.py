import itertools
import json
import sys

ROOT = "/code/shop"
LINES = [
    "    let total = cart.items.iter().map(|item| item.price * item.quantity).sum::<Money>();",
    "    if let Some(code) = cart.discount.as_ref() { total = code.apply(total)?; }",
    "fn checkout(cart: &Cart, payment: &dyn Payment) -> Result<Receipt, CheckoutError> {",
    "    tracing::debug!(items = cart.items.len(), \"starting checkout\");",
    "#[derive(Debug, Clone, PartialEq, Eq)]",
    "pub struct Receipt { pub id: OrderId, pub total: Money, pub paid_at: Timestamp }",
]


def text(chars, lines=LINES):
    out, size = [], 0
    for line in itertools.cycle(lines):
        if size >= chars:
            return "\n".join(out)
        out.append(line)
        size += len(line) + 1


def record(kind, **fields):
    return dict(type=kind, isSidechain=False, sessionId=sys.argv[2], **fields)


def attachment(**fields):
    return record("attachment", attachment=fields)


def user(content):
    return record("user", message=dict(role="user", content=content))


def assistant(blocks, total=None):
    message = dict(model="claude-opus-5-5", role="assistant", content=blocks)
    if total:
        message["usage"] = dict(input_tokens=4, cache_creation_input_tokens=1800, cache_read_input_tokens=total - 1804)
    return record("assistant", message=message)


def tool(counter, name, chars, **input):
    use = f"toolu_{next(counter):03}"
    result = user([dict(type="tool_result", tool_use_id=use, content=text(chars))])
    return [assistant([dict(type="tool_use", id=use, name=name, input=input)]), result]


def fixed():
    files = [("CLAUDE.md", "@AGENTS.md\n@DESIGN.md"), ("AGENTS.md", text(9000)), ("DESIGN.md", text(2200))]
    yield attachment(type="instructions", files=[
        dict(path=f"{ROOT}/{name}", type="Project", content=content) for name, content in files])
    yield attachment(type="skill_listing", content=text(5200))
    yield attachment(type="deferred_tools_delta", addedNames=["WebFetch", "WebSearch", "NotebookEdit", "TodoWrite"])
    yield attachment(type="mcp_instructions_delta", addedNames=["docs"], addedBlocks=[text(1600)])
    yield attachment(type="agent_listing_delta", addedTypes=["Explore", "Plan"], addedLines=[text(700), text(500)])
    yield attachment(type="prompt_snapshot", systemPrompt=[text(26000), text(4000)])


def conversation(counter):
    yield user("Apply discount codes at checkout and make the tests pass.")
    yield from tool(counter, "Read", 39000, file_path=f"{ROOT}/src/cart/checkout.rs")
    yield from tool(counter, "Read", 21000, file_path=f"{ROOT}/src/cart/mod.rs")
    yield from tool(counter, "Grep", 4800, pattern="Money::", path=f"{ROOT}/src")
    yield from tool(counter, "Grep", 2600, pattern="fn apply_discount")
    yield from tool(counter, "Glob", 1400, pattern="src/**/*.rs")
    yield assistant([dict(type="text", text=text(6000))])
    yield from tool(counter, "Read", 17000, file_path=f"{ROOT}/src/payment/stripe.rs")
    yield from tool(counter, "Read", 13500, file_path=f"{ROOT}/tests/checkout.rs")
    yield from tool(counter, "Edit", 5200, file_path=f"{ROOT}/src/cart/checkout.rs", old_string="", new_string="")
    yield from tool(counter, "Write", 4600, file_path=f"{ROOT}/src/cart/discount.rs", content=text(3000))
    yield from tool(counter, "Bash", 64000, command=f"cd {ROOT} && cargo test --locked")
    yield user("The flaky one is the stripe timeout test, skip it for now.")
    yield from tool(counter, "Bash", 11500, command="git diff")
    yield from tool(counter, "Bash", 7800, command="cargo clippy --all-targets -- -D warnings")
    yield from tool(counter, "Bash", 1500, command="git log --oneline -20")
    yield from tool(counter, "Agent", 6200, subagent_type="Explore", prompt="Where are discount codes validated?")
    yield from tool(counter, "TodoWrite", 1900, todos=[])
    yield from tool(counter, "WebFetch", 3800, url="https://docs.example.com/payments/discounts")
    yield assistant([dict(type="text", text=text(9000))], total=124000)


def main(path):
    counter = itertools.count(1)
    before = [user("Look around the cart module."), *tool(counter, "Read", 30000, file_path=f"{ROOT}/src/lib.rs")]
    boundary = dict(type="system", subtype="compact_boundary", content="Conversation compacted", isSidechain=False)
    records = [*fixed(), *before, boundary, *conversation(counter)]
    with open(path, "w") as file:
        file.writelines(json.dumps(r) + "\n" for r in records)


if __name__ == "__main__":
    main(sys.argv[1])
