# FIRESTORM_AVATAR_PROFILE_MAP.md

Firestorm behavior reference for Avatar Profiles, used to guide this Rust viewer implementation.

## UI Tab Handles
- `panel_profile_secondlife`
- `panel_profile_web`
- `panel_profile_picks`
- `panel_profile_classifieds`
- `panel_profile_firstlife`
- `panel_profile_notes`

Primary floater:
- `floater_profile.xml` (`panel_profile_tabs`)

## Command Handlers And Verbs
- `profile` handler:
  - opens web profile URL by name/route.
- `agent` / `agentself` handlers:
  - verbs include `about`, `inspect`, `im`, `pay`, `offerteleport`, `requestfriend`, `removefriend`, `mute`, `unmute`, `reportAbuse`.
- `pick` handler:
  - `create`
  - `{pick_id}/edit`
- `classified` handler:
  - `create`
  - `{classified_id}/about`
  - `{classified_id}/edit`

## LLSD Key Parameters
- Floater open key:
  - `id`
- Classified panel open keys:
  - `classified_creator_id`
  - `classified_id`
  - `classified_name`
  - `from_search`
  - `edit`

## Capability Names Used In Profile Flows
- `AgentProfile`
- `UploadAgentProfileImage`
- `SearchStatRequest`
- `SearchStatTracking`
- `GetDisplayNames`

## Generic Methods And UDP Messages

Generic request methods:
- `avatarpicksrequest`
- `avatarclassifiedsrequest`
- `avatarnotesrequest`
- `avatargroupsrequest`
- `pickinforequest`

UDP profile-related messages (from `message_template.msg`):
- `AvatarPropertiesRequest` / `AvatarPropertiesReply`
- `AvatarGroupsReply`
- `AvatarPicksReply`
- `AvatarNotesReply`
- `AvatarClassifiedReply`
- `PickInfoReply`
- `ClassifiedInfoReply`

Related update/delete/profile-control messages (documented for future editable phases):
- `AvatarPropertiesUpdate`
- `AvatarNotesUpdate`
- `PickInfoUpdate`
- `PickDelete`
- `ClassifiedInfoUpdate`
- `ClassifiedDelete`
- `GrantUserRights`

## Firestorm Source Anchors
- `reference/firestorm/indra/newview/llfloaterprofile.cpp`
- `reference/firestorm/indra/newview/llpanelprofile.cpp`
- `reference/firestorm/indra/newview/llpanelprofilepicks.cpp`
- `reference/firestorm/indra/newview/llpanelprofileclassifieds.cpp`
- `reference/firestorm/indra/newview/llavatarpropertiesprocessor.cpp`
- `reference/firestorm/indra/newview/llpanelavatar.cpp`
- `reference/firestorm/scripts/messages/message_template.msg`
