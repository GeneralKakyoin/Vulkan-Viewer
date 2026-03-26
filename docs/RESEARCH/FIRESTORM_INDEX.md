# FIRESTORM_INDEX.md

Task-oriented index for the local Firestorm reference tree at `reference/firestorm/`.

## Login + Bootstrap
- Login request/response shaping:
  - `reference/firestorm/indra/newview/llstartup.cpp`
  - `reference/firestorm/indra/newview/llappviewer.cpp`
- Login wire and options context:
  - `reference/firestorm/indra/llmessage/lllogininstance.cpp`
  - `reference/firestorm/indra/llmessage/lllogininstance.h`
- Message IDs:
  - `reference/firestorm/scripts/messages/message_template.msg`

## Nearby Chat
- Nearby send path (`ChatFromViewer`):
  - `reference/firestorm/indra/newview/llfloaterimnearbychat.cpp`
  - `reference/firestorm/indra/newview/fsnearbychathub.cpp`
- Nearby receive path (`ChatFromSimulator`):
  - `reference/firestorm/indra/newview/llviewermessage.cpp` (`process_chat_from_simulator`)
- Message template IDs/fields:
  - `reference/firestorm/scripts/messages/message_template.msg`
  - `ChatFromViewer` low `80`
  - `ChatFromSimulator` low `139`

## Friends Bootstrap + Presence
- Login bootstrap friend list (`buddy-list`) handling:
  - `reference/firestorm/indra/newview/llstartup.cpp`
- Friend tracking and runtime callbacks:
  - `reference/firestorm/indra/newview/llcallingcard.cpp`
  - `reference/firestorm/indra/newview/llcallingcard.h`
- Presence/rights messages:
  - `OnlineNotification` low `322`
  - `OfflineNotification` low `323`
  - `ChangeUserRights` low `321`
  - Source: `reference/firestorm/scripts/messages/message_template.msg`

## Direct IM Send/Receive
- Core IM message packing:
  - `reference/firestorm/indra/llmessage/llinstantmessage.cpp`
- Runtime IM processing:
  - `reference/firestorm/indra/newview/llviewermessage.cpp` (`process_improved_im`)
- Session + IM behaviors:
  - `reference/firestorm/indra/newview/llimview.cpp`
  - `LLIMMgr::computeSessionID` behavior for P2P/group/ad-hoc
- Message template IDs/fields:
  - `ImprovedInstantMessage` low `254`
  - `RetrieveInstantMessages` low `255`
  - Source: `reference/firestorm/scripts/messages/message_template.msg`

## Friend Name Resolution
- Name cache and async display-name updates:
  - `reference/firestorm/indra/newview/llavatarnamecache.cpp`
  - `reference/firestorm/indra/newview/llavatarnamecache.h`
- IM-originated name hints that can populate UI labels:
  - `reference/firestorm/indra/newview/llviewermessage.cpp` (`process_improved_im`)

## Avatar Presence + Coarse Locations
- Coarse presence decode and nearby avatar indexing:
  - `reference/firestorm/indra/newview/llviewerregion.cpp`
  - `reference/firestorm/indra/newview/llworld.cpp`
  - `reference/firestorm/indra/newview/llviewermessage.cpp`
- Message template:
  - `CoarseLocationUpdate` medium `6`
  - `RegionHandshake` low `148` (region/sim metadata, including `SimName`)
  - `AgentMovementComplete` low `250` (local position + region handle)
  - source: `reference/firestorm/scripts/messages/message_template.msg`
- Practical notes for this repo:
  - prefer coarse payload IDs when present
  - keep self-avatar fallback even when coarse ID blocks are missing
  - derive current sim name from `RegionHandshake` and local position from `AgentMovementComplete` for viewer-visible location context

## Avatar Profiles
- Task-oriented profile mapping (handles, parameters, caps, message IDs):
  - `docs/FIRESTORM_AVATAR_PROFILE_MAP.md`
- Core profile floater + panels:
  - `reference/firestorm/indra/newview/llfloaterprofile.cpp`
  - `reference/firestorm/indra/newview/llpanelprofile.cpp`
  - `reference/firestorm/indra/newview/llpanelprofilepicks.cpp`
  - `reference/firestorm/indra/newview/llpanelprofileclassifieds.cpp`
- Profile transport processor:
  - `reference/firestorm/indra/newview/llavatarpropertiesprocessor.cpp`
  - `reference/firestorm/indra/newview/llavatarpropertiesprocessor.h`
- UI XML tab structure:
  - `reference/firestorm/indra/newview/skins/default/xui/en/floater_profile.xml`
- Legacy UDP/generic fallback paths used by profile tabs:
  - Generic methods: `avatarpicksrequest`, `pickinforequest`, `avatarclassifiedsrequest`, `avatarnotesrequest`, `avatargroupsrequest`
  - Replies: `AvatarPropertiesReply` (`171`), `AvatarGroupsReply` (`173`), `AvatarNotesReply` (`176`), `AvatarPicksReply` (`178`), `PickInfoReply` (`184`), `AvatarClassifiedReply` (`42`), `ClassifiedInfoReply` (`44`)
  - Request IDs: `AvatarPropertiesRequest` (`169`), `ClassifiedInfoRequest` (`43`), `GenericMessage` (`261`)

## Fast Lookup: Message Template
- Primary file:
  - `reference/firestorm/scripts/messages/message_template.msg`
- Useful sections:
  - Instant messaging block (`ImprovedInstantMessage`, `RetrieveInstantMessages`)
  - Social notifications (`ChangeUserRights`, `OnlineNotification`, `OfflineNotification`)
  - Nearby chat (`ChatFromViewer`, `ChatFromSimulator`)
