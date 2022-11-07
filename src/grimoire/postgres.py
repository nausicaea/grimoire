from contextlib import asynccontextmanager
from collections.abc import AsyncGenerator

try:
    import asyncpg


    @asynccontextmanager
    async def connect(dsn: str, password: str) -> AsyncGenerator[asyncpg.Connection, None]:
        con: asyncpg.Connection = await asyncpg.connect(dsn, password=password)
        try:
            yield con
        finally:
            if con is not None:
                await con.close()

except ImportError as ex:
    asyncpg = ex

