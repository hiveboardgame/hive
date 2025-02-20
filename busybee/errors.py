from discord.ext import commands
from discord.ext.commands.errors import *
from constants import *
from utils import pretty_print
import sys
import traceback


class DatabaseError(commands.CommandError):
    def __init__(self, message, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.message = message


class UserNotLinked(commands.CommandError):
    def __init__(self, message, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.message = message


class AuthenticationFailure(commands.CommandError):
    def __init__(self, message, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.message = message


class MessageSendFailure(commands.CommandError):
    def __init__(self, message, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.message = message


class IllegalRole(commands.CommandError):
    def __init__(self, message, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.message = message


def standard_error_handler(error_function):
    """
    Decorator that is prepended to a cog_command_error.
    It transforms the error handler into one that makes helpful and specific checks
    and returns a function that readily prints the error
    """

    async def wrapper(cls, ctx, error):

        extra = f"\n\nSee the help message for more information."

        # Do not interfere with discord.py's internal error handling
        if hasattr(ctx.command, "on_error"):
            return

        # Allows us to check for original exceptions raised and sent to CommandInvokeError.
        # If nothing is found. We keep the exception passed to on_command_error.
        error = getattr(error, "original", error)

        # Don't bother handling the following errors
        ignored = (commands.CommandNotFound,)
        if any([isinstance(error, i) for i in ignored]):
            return

        elif isinstance(error, MissingAnyRole):
            await pretty_print(ctx, str(error), title="Error", color=ERROR_COLOR)
            return

        elif isinstance(error, DisabledCommand):
            await pretty_print(
                ctx, "This command is disabled!", title="Error", color=ERROR_COLOR
            )
            return

        elif isinstance(error, NoPrivateMessage):
            await pretty_print(
                ctx,
                "This command cannot be run in a private message." + extra,
                title="Error",
                color=ERROR_COLOR,
            )
            return

        elif isinstance(error, PrivateMessageOnly):
            try:
                await ctx.message.delete()
                extra += "\nYour message has been deleted"
            except:
                print("Could not delete message")
            await pretty_print(
                ctx,
                "This command should be run in a Private Message only!" + extra,
                title="Error",
                color=ERROR_COLOR,
            )
            return

        elif isinstance(error, MissingRole):
            await pretty_print(
                ctx, str(error) + extra, title="Error", color=ERROR_COLOR
            )
            return

        elif isinstance(error, IllegalRole):
            await pretty_print(
                ctx, error.message + extra, title="Error", color=ERROR_COLOR
            )
            return

        elif isinstance(error, MessageSendFailure):
            await pretty_print(
                ctx,
                "Could not send message to user" + extra,
                title="Error",
                color=ERROR_COLOR,
            )
            return

        elif isinstance(error, CheckFailure):
            await pretty_print(
                ctx,
                "Could not run command, do you have sufficient permissions in this channel?"
                + extra,
                title="Error",
                color=ERROR_COLOR,
            )
            return

        elif isinstance(error, BadArgument):
            await ctx.send_help(ctx.command)
            await pretty_print(ctx, str(error), title="Error", color=ERROR_COLOR)
            return

        elif isinstance(error, commands.MissingRequiredArgument):
            await ctx.send_help(ctx.command)
            await pretty_print(
                ctx,
                "Missing required arguments\n" + str(error),
                title="Error",
                color=ERROR_COLOR,
            )
            return

        await error_function(cls, ctx, error)

    return wrapper
