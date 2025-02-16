import json
import discord
import time
import secrets
import random
import config
import constants
import logging

config.init()
logger = logging.getLogger(__name__)


class LazyDatabaseModel:
    """
    This class abstracts the implementation details of storing to a database
    and creates python objects from SQL data and vice versa easily.
    """

    model_name = "generic"

    # These are internal variables that are not saved to the database
    unsaved_vars = []

    def __init__(self, unique_id=None):
        self.unique_id = unique_id

    @classmethod
    def count(cls):
        return len(config.db[cls.model_name])

    # Save model to database
    def save_to_database(self):
        assert self.model_name != "generic", "The model name cannot be generic!!!"
        result = self.to_dict()

        assert "unique_id" in result, "This model has no unique id"
        assert result["unique_id"] is not None, "This model's unique id is empty"

        if "id" in result:
            del result["id"]

        config.db[self.model_name].upsert(result, ["unique_id"])
        config.db.commit()
        return self

    @classmethod
    def cast_type(self, **params):
        if params.get("username", None):
            params["username"] = params["username"].lower()
        if params.get("item_id", None):
            params["item_id"] = str(params["item_id"])
        return params

    @classmethod
    def delete_all_from_database(cls):
        was_deleted_successfully = config.db[cls.model_name].delete()

    @classmethod
    def delete_from_database(cls, **search_args):
        was_deleted_successfully = config.db[cls.model_name].delete(**search_args)
        if not search_args:
            logger.error(
                "Suppressed deletion of all records in table, use delete_all explicity if you want to do that!!!"
            )
            return False

        config.db.commit()
        return was_deleted_successfully

    @classmethod
    def find_one(cls, **search_args):
        search_args = cls.cast_type(**search_args)
        result = config.db[cls.model_name].find_one(**search_args)
        if not result:
            return None
        if "id" in result:
            del result["id"]

        result = {k: cls.unpack(v) for k, v in result.items()}
        return cls(**result)

    def to_dict(self):

        # Collect all static variables
        ret_dict = vars(self)

        # Ignore any variable with a "_" prefix (hidden variables by Python convention)
        ret_dict = {k: v for k, v in ret_dict.items() if not k.startswith("_")}

        # Ignore any variable specified by unsaved_vares
        ret_dict = {
            k: self.pack(v) for k, v in ret_dict.items() if k not in self.unsaved_vars
        }

        assert "id" not in ret_dict, "Cannot use reserved keyword id as an argument"

        return ret_dict

    @classmethod
    def from_dict(cls, dictionary):
        for item_name, value in dictionary.items():
            return cls(**dictionary)

    @staticmethod
    def pack(value):
        if type(value) in [set, dict, list, tuple]:
            if type(value) == set:
                value = list(value)
            return json.dumps(value)
        return value

    @staticmethod
    def unpack(value):
        if type(value) == str:
            try:
                return json.loads(value)
            except:
                pass
        return value

    @classmethod
    def find(cls, **kwargs):
        search_args = cls.cast_type(**kwargs)
        models = []
        results = config.db[cls.model_name].find(**kwargs)

        for result in results:
            if "id" in result:
                del result["id"]
            result = {k: cls.unpack(v) for k, v in result.items()}
            model = cls(**result)
            models.append(model)
        return models


class UserRecord(LazyDatabaseModel):
    """
    Stores linked users, linked via discord snowflake id and hive user uuid
    """

    model_name = "user_record"

    def __init__(self, discord_user_id=None, hive_user_id=None, unique_id=None,  avatar_url=None, username=None):
        assert (
            int(discord_user_id) == discord_user_id
        ), "Discord user id must be an integer"
        assert hive_user_id is not None, "Hive user id missing"
        self.unique_id = unique_id or discord_user_id
        self.discord_user_id = int(discord_user_id)
        self.hive_user_id = str(hive_user_id)
        self.avatar_url = avatar_url
        self.username = username


class UserPreferences(LazyDatabaseModel):
    """
    Stores linked user preferences, should have a one to one relationship with
    UserRecords
    """

    model_name = "user_preferences"

    def __init__(self, discord_user_id=None, preferences=None, unique_id=None):
        assert (
            int(discord_user_id) == discord_user_id
        ), "Discord user id must be an integer"
        self.unique_id = unique_id or discord_user_id
        self.discord_user_id = int(discord_user_id)
        prefs = default_prefs()
        prefs.update(preferences or {})

        self.preferences = prefs

    def pings_enabled(self):
        return self.preferences.get("pings_enabled", False)

    def send_pings_to_dm_enabled(self):
        return self.preferences.get("send_pings_to_dm_enabled", False)

    def set_pings_enabled(self, enabled: bool):
        self.preferences["pings_enabled"] = enabled

    def set_send_pings_to_dm_enabled(self, enabled: bool):
        self.preferences["send_pings_to_dm_enabled"] = enabled


# Note: For hassle free migration, always *add* new default preferences
# and never remove them.
def default_prefs():
    return {
        "send_pings_to_dm_enabled": True,
        "pings_enabled": False,
    }


class OauthState(LazyDatabaseModel):
    """
    Stores generated states for users, used to verify that the user is the same
    one that initiated the oauth flow
    """

    model_name = "oauth_state"

    def __init__(self, hive_user_id=None, token=None, expires=None, unique_id=None):
        assert hive_user_id is not None, "Hive user id missing"
        self.unique_id = unique_id or hive_user_id
        self.hive_user_id = hive_user_id
        self.token = token or "t01" + secrets.token_urlsafe(64)
        self.expires = expires or int(time.time()) + constants.OAUTH_SECRET_EXPIRY

    @staticmethod
    def is_valid(token):
        oauth_state = OauthState.find_one(token=token)

        if not oauth_state:
            return False
        if oauth_state.expires < time.time():
            return False
        return True

    @staticmethod
    def generate_token(hive_user_id):
        state = OauthState(hive_user_id=hive_user_id)
        state.save_to_database()
        return state.token
