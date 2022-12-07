import asyncio.subprocess


async def recall(item: str, field: str, vault: str | None = None) -> str:
    """
    Recall knowledge from the abyss by calling on the augurs of onepassword.

    >>> import asyncio
    >>> from grimoire.onepassword import recall
    >>> asyncio.run(recall('Retrieval Marker', 'password', vault='Temporary Items'))
    'q47h9HUmTdC2PBycx24znF2PHgpyYdJT'
    """
    args = [
        "op",
        "item",
        "get",
        f"--fields=label={field}",
        item,
    ]

    if vault is not None:
        args.insert(3, f"--vault={vault}")

    p = await asyncio.subprocess.create_subprocess_exec(
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    stdout, stderr = await p.communicate()

    if p.returncode != 0:
        raise RuntimeError(stderr.decode("utf-8"))

    return stdout.decode("utf-8").strip()
