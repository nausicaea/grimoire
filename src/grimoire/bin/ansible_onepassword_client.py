#!/usr/bin/env python3
import argparse

from grimoire.invocation import start
from grimoire.onepassword import recall


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
        required=True,
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
