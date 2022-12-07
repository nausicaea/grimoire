import asyncio.subprocess

import pytest


@pytest.mark.asyncio
async def test_ansible_onepassword_client_is_callable_from_cli() -> None:
    p = await asyncio.subprocess.create_subprocess_exec(
        "ansible-onepassword-client",
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    _, stderr = await p.communicate()

    assert p.returncode == 2
    assert stderr.decode("utf-8").startswith("usage: ansible-onepassword-client")
