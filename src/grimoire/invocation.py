import asyncio
import functools
import sys
from typing import Callable, Coroutine


def start(coro: Callable[[], Coroutine]) -> Callable[[], None]:
    """
    Wraps asynchronous invocation functions such that you may call them from a synchronous context.
    Use this to easily create an asynchronous main function, for example.

    >>> import asyncio
    >>>
    >>> @start
    >>> async def main() -> None:
    >>>   asyncio.sleep(1)
    >>>   print('Hello, World')
    >>>
    >>> main()
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
