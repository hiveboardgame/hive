import config
import discord
from constants import BOT_NAME


async def pretty_print(
    ctx,
    fields,
    caption="",
    title="",
    author=BOT_NAME,
    color=discord.Color(0xFFFFFF),
):
    """
    A method for printing to the Discord channel with a custom embed.

    Parameters
    __________

      ctx (discord.Context) – The invocation context where the call was made
      fields (list or string) - Either a comma separated list of fields or a single string
                                Each field is organized by [Title, Value, Inline] as specified in Discord documentation
      caption (string) - A message to append to the bottom of the embed, useful for printing mentions and such
      title (string) - Title listed at the top of the embed
      author  - The author of this message
      color (discord.Color) - A object representing the color strip on the left side of the Embed

    """

    if not ctx:
        return

    if not fields:
        fields = "..."

    groups = [fields]
    max_string_len = 1000
    max_list_len = 20
    if type(fields) == list and len(fields) > max_list_len:
        groups = [
            fields[i : i + max_list_len] for i in range(0, len(fields), max_list_len)
        ]
    if type(fields) == str and len(fields) >= max_string_len:
        groups = [
            fields[i : i + max_string_len]
            for i in range(0, len(fields), max_string_len)
        ]

    for index, fields in enumerate(groups):
        embed = discord.Embed(title=title, color=color)

        if author:
            embed.set_author(name=author)

        if type(fields) == list:
            for field in fields:
                if len(field) < 3:
                    field.append(True)

                name, value, inline = field
                if not value:
                    value = "..."
                embed.add_field(name=name, value=value, inline=inline)

        elif type(fields) == str:
            if index < len(groups) - 1:
                fields += "..."
            if index > 0:
                fields = "..." + fields
            embed.add_field(name="-------------", value=fields)

        if caption:
            await ctx.send(content=caption, embed=embed)
        else:
            await ctx.send(embed=embed)
