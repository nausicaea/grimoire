import asyncio
import functools
import sys
from typing import Any, Callable, Coroutine


def start(coro: Callable[[], Coroutine[Any, Any, Any]]) -> Callable[[], None]:
    """
    Wraps asynchronous invocation functions such that you may call them from a synchronous context.
    Use this to easily create an asynchronous main function, for example.

    >>> import asyncio
    >>> from grimoire.invocation import start
    >>> @start
    ... async def main() -> None:
    ...     await asyncio.sleep(1)
    ...     print('Hello, World')
    >>> main()
    Hello, World
    >>>
    """

    @functools.wraps(coro)
    def wrapper() -> None:
        if sys.version_info >= (3, 7):
            asyncio.run(coro())
        else:
            loop = asyncio.get_event_loop()
            loop.run_until_complete(coro())
            if not loop.is_closed():
                loop.close()

    return wrapper
