try:
    from asyncio import AbstractEventLoop
    from collections.abc import AsyncGenerator, Sequence
    from contextlib import asynccontextmanager
    from typing import Literal, Type

    import asyncpg
    import asyncpg.connect_utils
    import asyncpg.protocol

    @asynccontextmanager
    async def connect(
        dsn: str | list[str] | None = None,
        *,
        host: str | list[str] | None = None,
        port: int | str | list[int] | None = None,
        user: str | None = None,
        password: str | None = None,
        passfile: str | None = None,
        database: str | None = None,
        loop: AbstractEventLoop | None = None,
        timeout: int | float = 60,
        statement_cache_size: int = 100,
        max_cached_statement_lifetime: int | float = 300,
        max_cacheable_statement_size: int = 1024 * 15,
        command_timeout: int | float | None = None,
        ssl: Literal[True] | str | asyncpg.connect_utils.SSLMode | None = None,
        direct_tls: bool = False,
        connection_class: Type[asyncpg.Connection] = asyncpg.Connection,
        record_class: Type[asyncpg.protocol.Record] = asyncpg.protocol.Record,
        server_settings: dict[str, str] | None = None,
    ) -> AsyncGenerator[asyncpg.Connection, None]:
        """
        Open a connection to Postgres by way of context manager. This ensures that any remaining traces of a connection
        are closed up even if something bad has happened in-between.

        >>> import asyncio
        >>> import asyncio.subprocess
        >>> from grimoire.postgres import connect
        >>> async def main() -> None:
        ...     p = await asyncio.subprocess.create_subprocess_exec(*['podman', 'container', 'run', '-d', '--rm', '-e', 'POSTGRES_HOST_AUTH_METHOD=trust', '-p', '5432:5432', 'docker.io/library/postgres:latest'], stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        ...     stdout, stderr = await p.communicate()
        ...     if p.returncode != 0:
        ...         raise RuntimeError(stderr)
        ...     container_id: str = stdout.decode('utf-8').strip()
        ...     await asyncio.sleep(10)
        ...     async with connect('postgres://postgres@[::1]:5432/postgres') as conn:
        ...         stmt = await conn.prepare('SELECT 1 + $1')
        ...         result = await stmt.fetchval(12)
        ...         print(result)
        ...     p = await asyncio.subprocess.create_subprocess_exec(*['podman', 'container', 'stop', container_id])
        ...     await p.wait()
        >>> asyncio.run(main())
        13
        """
        con: asyncpg.Connection = await asyncpg.connect(
            dsn,
            host=host,
            port=port,
            user=user,
            password=password,
            passfile=passfile,
            database=database,
            loop=loop,
            timeout=timeout,
            statement_cache_size=statement_cache_size,
            max_cached_statement_lifetime=max_cached_statement_lifetime,
            max_cacheable_statement_size=max_cacheable_statement_size,
            command_timeout=command_timeout,
            ssl=ssl,
            direct_tls=direct_tls,
            connection_class=connection_class,
            record_class=record_class,
            server_settings=server_settings,
        )
        try:
            yield con
        finally:
            if con is not None:
                await con.close()

except ImportError as ex:
    asyncpg = ex
