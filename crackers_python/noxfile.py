import nox

@nox.session
def ruff(session):
    """Run ruff linter."""
    session.install("ruff")
    session.run("ruff", "check", "crackers")

@nox.session
def mypy(session):
    """Run mypy type checker."""
    session.install("mypy")
    session.run("mypy", "crackers")
