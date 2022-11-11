#!/usr/bin/env python3
import argparse
import asyncio.subprocess

from grimoire.invocation import start


async def recall(item_id: str, field: str) -> str:
    """
    Recall knowledge from the abyss by calling on the augurs of onepassword.

    >>> import asyncio
    >>> from grimoire.onepassword import recall
    >>> asyncio.run(recall('Status', 'name'))
    """
    p = await asyncio.subprocess.create_subprocess_exec(
        *[
            "op",
            "item",
            "get",
            item_id,
            "--fields",
            f"label={field}",
        ],
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    stdout, stderr = await p.communicate()

    if p.returncode != 0:
        raise RuntimeError(stderr.decode("utf-8"))

    return stdout.decode("utf-8").strip()


@start
async def main() -> None:
    """
    Recall One Password vault items in a way that is compatible with Ansible Vault
    """
    parser = argparse.ArgumentParser(
        description="Recall One Password vault items in a way that is compatible with Ansible Vault"
    )
    parser.add_argument(
        "--vault-id",
        type=str,
        help="Supply the name or ID of the vault item in One Password. To Ansible, this identifier is known as the Ansible Vault ID.",
    )
    parser.add_argument(
        "--field",
        type=str,
        default="credential",
        help='Select the vault item field that is returned. Default: "credential"',
    )
    matches = parser.parse_args()

    vault_id: str = matches.vault_id
    field: str = matches.field

    item_data = await recall(vault_id, field)

    print(item_data)


if __name__ == "__main__":
    main()
