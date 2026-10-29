"""Check rulebook evidence listings against independently scanned S0 source values."""

from pathlib import Path

from pydantic import BaseModel, ConfigDict
from rulebook_fixture_support import (
    CONFIG,
    SKILL,
    JsonObject,
    array_value,
    object_value,
    oracle_matches,
    read_object,
    require,
)


class EvidenceListing(BaseModel):
    """Caller view of sunstack-lint's source-declared EvidenceListing envelope."""

    model_config = ConfigDict(extra="ignore")
    version: str
    config: str
    scope: JsonObject
    pieces: list[JsonObject]
    usage: JsonObject


def listing_pieces(raw: bytes) -> list[JsonObject]:
    return EvidenceListing.model_validate_json(raw).pieces


def location(piece: JsonObject) -> JsonObject:
    return object_value(piece["where"])


def signature(piece: JsonObject) -> tuple[str, str, int, int]:
    where = location(piece)
    span = object_value(where["span"])
    start, end = span["byte_start"], span["byte_end"]
    require(type(start) is int and type(end) is int, "Piece offsets must be integers")
    return str(piece["kind"]), str(where["file"]), int(str(start)), int(str(end))


def require_piece_contract(pieces: list[JsonObject], root: Path) -> None:
    config = read_object(root / CONFIG)
    kinds = object_value(object_value(config["evidence"])["kinds"])
    matches = oracle_matches(root)
    expected: list[tuple[str, str, int, int]] = []
    for kind, raw in kinds.items():
        rule = object_value(raw)["rule"]
        expected.extend(
            (kind, str(item["file"]), int(str(item["start"])), int(str(item["end"])))
            for item in matches
            if item["rule"] == rule and str(item["file"]).endswith("/SKILL.md")
        )
    require(
        [signature(piece) for piece in pieces] == sorted(expected),
        "Pieces differ from independent scan oracle or contracted order",
    )
    selected = [
        piece for piece in pieces if signature(piece) == ("step", SKILL, 6114, 7909)
    ]
    require(len(selected) == 1, "Step 3 was split or lost")
    piece = selected[0]
    require(
        location(piece)["first_line"] == 49 and location(piece)["last_line"] == 60,
        "Step 3 line range differs",
    )
    fields = object_value(piece["fields"])
    source_lines = (root / SKILL).read_text().splitlines()
    heading = object_value(fields["heading"])
    require(
        heading["state"] == "text" and heading["value"] == "3. Choose each observation",
        "Step heading differs",
    )
    completion = object_value(fields["completion"])
    require(completion["state"] == "texts", "Completion lost its list state")
    spans = array_value(completion["spans"])
    require(
        len(spans) == 1 and object_value(spans[0])["start_line"] == 59,
        "Completion attached to another step",
    )
    require(
        completion["values"]
        == [
            item["text"]
            for item in matches
            if item["rule"] == "completion"
            and item["file"] == SKILL
            and int(str(item["start"])) >= 6114
            and int(str(item["end"])) <= 7909
        ],
        "Completion text differs from line 59",
    )
    body = object_value(fields["body"])
    require(
        body["state"] == "text"
        and body["value"] == "\n".join(source_lines[50:57]).strip(),
        "Body grouping differs",
    )
    loaded = [item for item in pieces if signature(item) == ("load", SKILL, 6146, 6445)]
    require(len(loaded) == 1, "Line 51 load absent")
    targets = array_value(
        object_value(object_value(loaded[0]["fields"])["targets"])["values"]
    )
    target = next(
        (
            object_value(item)
            for item in targets
            if object_value(item)["name"] == "references/choosing-the-observation.md"
        ),
        None,
    )
    require(
        target is not None and target["exists"] is True, "Load target did not resolve"
    )
    if target is not None:
        opening = (
            root / "skills/verification-design/references/choosing-the-observation.md"
        ).read_text()[:3000]
        require(
            target["opening"] == opening, "Load opening differs from its actual target"
        )
    retrospective = next(
        (
            item
            for item in pieces
            if item["kind"] == "load"
            and location(item)["file"] == "skills/practices-retrospective/SKILL.md"
            and location(item)["first_line"] == 21
        ),
        None,
    )
    require(retrospective is not None, "Retrospective skill load is missing")
    if retrospective is not None:
        loaded_skills = array_value(
            object_value(object_value(retrospective["fields"])["targets"])["values"]
        )
        research = next(
            (
                object_value(item)
                for item in loaded_skills
                if object_value(item)["name"] == "practices-research"
            ),
            None,
        )
        require(
            research is not None and research["exists"] is True,
            "Named skill did not resolve",
        )
        if research is not None:
            nested = object_value(research["fields"])
            require(
                object_value(nested["description"])["state"] == "text"
                and "targets" not in nested,
                "Named skill fields were lost or composition became recursive",
            )
    skill_piece = next(
        item
        for item in pieces
        if item["kind"] == "skill" and location(item)["file"] == SKILL
    )
    description = object_value(object_value(skill_piece["fields"])["description"])
    require(
        description["state"] == "text" and len(str(description["value"])) == 450,
        "Description YAML value differs",
    )
