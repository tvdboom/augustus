//! Bevy adapter for online Roman campaigns and the existing local UI.
use super::*;
use crate::multiplayer::{
    model::*,
    patch,
    supabase::{self, Response, SupabaseLobbyAdapter},
};
use crate::platform::storage::{self, Session};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{mpsc, Mutex};

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    campaign: campaign::Campaign,
    clock: GameClock,
    paused: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    Create,
    Join,
    Open,
    LoadStarted,
    Recover,
    Profile,
    Start,
    Resume,
    List,
    Save,
    Sync,
    Leave,
}

struct Pending {
    kind: Operation,
    payload: Value,
    receiver: Mutex<mpsc::Receiver<Response>>,
    sent: Option<Value>,
}

#[derive(Resource)]
pub(super) struct OnlineClient {
    transport: Result<SupabaseLobbyAdapter, String>,
    session: Option<Session>,
    connection: String,
    pub record: Option<GameRecord>,
    base: Option<Value>,
    pending: Option<Pending>,
    queue: VecDeque<(Operation, Value)>,
    pub games: Vec<GameSummary>,
    error: Option<String>,
    pending_errors: VecDeque<String>,
    pub notice: Option<String>,
    elapsed: f32,
    failures: u32,
    since_sync: f32,
    save_requested: bool,
    leave_requested: bool,
    reconcile_from: Option<Value>,
    retry_sent: Option<Value>,
    pub connected: bool,
    reconnect_lobby: bool,
    resume_generation: u64,
}

impl Default for OnlineClient {
    fn default() -> Self {
        Self {
            transport: SupabaseLobbyAdapter::configured(),
            session: storage::load(),
            connection: supabase::request_id(),
            record: None,
            base: None,
            pending: None,
            queue: VecDeque::new(),
            games: vec![],
            error: None,
            pending_errors: VecDeque::new(),
            notice: None,
            elapsed: 0.,
            failures: 0,
            since_sync: 0.,
            save_requested: false,
            leave_requested: false,
            reconcile_from: None,
            retry_sent: None,
            connected: false,
            reconnect_lobby: false,
            resume_generation: 0,
        }
    }
}

impl OnlineClient {
    /// Announce each failure once until recovery or an explicit retry.
    pub fn report_error(&mut self, error: impl Into<String>) {
        let error = error.into();
        if self.error.as_ref() != Some(&error) {
            self.pending_errors.push_back(error.clone());
        }
        self.notice = None;
        self.error = Some(error);
    }

    pub fn take_errors(&mut self) -> impl Iterator<Item = String> + '_ {
        self.pending_errors.drain(..)
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn list_pending(&self) -> bool {
        self.pending.as_ref().is_some_and(|p| p.kind == Operation::List)
            || self.queue.iter().any(|(kind, _)| *kind == Operation::List)
    }

    pub fn reconnecting(&self) -> bool {
        self.reconnect_lobby
    }

    pub fn resume(&mut self) {
        if let Some(record) = &self.record {
            self.request(Operation::Resume, json!({"p_roster_token":record.roster_token}));
        }
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some() || !self.queue.is_empty() || self.leave_requested
    }

    /// Background polling keeps menu actions available; writes still serialize.
    pub fn foreground_busy(&self) -> bool {
        self.pending.as_ref().is_some_and(|p| p.kind != Operation::Sync)
            || !self.queue.is_empty()
            || self.leave_requested
    }

    pub fn request(&mut self, kind: Operation, mut payload: Value) {
        if self.foreground_busy() || (kind == Operation::Sync && self.busy()) {
            return;
        }
        if kind != Operation::List {
            payload["p_connection"] = json!(self.connection);
        }
        if let Some(record) = &self.record {
            if !matches!(
                kind,
                Operation::Create
                    | Operation::Join
                    | Operation::Recover
                    | Operation::Open
                    | Operation::List
            ) {
                payload["p_game_id"] = json!(record.id);
            }
        }
        if matches!(kind, Operation::Create | Operation::Resume) {
            payload["p_request_id"] = json!(supabase::request_id());
        }
        self.error = None;
        self.notice = None;
        self.queue.push_back((kind, payload));
    }
    pub fn save(&mut self) {
        self.error = None;
        self.save_requested = true;
        self.elapsed = 30.;
    }
    pub fn leave(&mut self) {
        self.leave_requested = true;
        self.save();
    }
    pub fn host(&self) -> bool {
        self.record.as_ref().is_none_or(|r| r.player == 0)
    }
    pub fn can_save(&self) -> bool {
        self.record.as_ref().is_some_and(|r| r.status == "active" && !self.eliminated(r.player))
    }
    pub fn runs_time(&self) -> bool {
        self.record.as_ref().is_none_or(|r| {
            r.status == "active"
                && r.player == 0
                && self.connected
                && self.error.is_none()
                && !self.reconnect_lobby
                && !self.busy()
                && r.members.iter().all(|m| m.connected || self.eliminated(m.player))
        })
    }
    pub fn start(&mut self) -> Result<(), String> {
        let record = self.record.as_ref().ok_or("No lobby is open.")?;
        let mut ownership = ProvinceOwnership::default();
        ownership
            .start_game(&record.members.iter().map(|m| PLAYER_COLORS[m.color]).collect::<Vec<_>>());
        let mut campaign = campaign::Campaign::default();
        campaign.start(&ownership, record.members.len());
        let snapshot = Snapshot {
            version: 1,
            campaign,
            clock: GameClock::default(),
            paused: false,
        };
        let state =
            serde_json::to_value(snapshot).map_err(|_| "Could not prepare the campaign.")?;
        self.request(
            Operation::Start,
            json!({"p_roster_token":record.roster_token,"p_state":state}),
        );
        Ok(())
    }
    fn eliminated(&self, player: usize) -> bool {
        self.base
            .as_ref()
            .and_then(|s| s["campaign"]["defeated"].get(player))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

fn snapshot(world: &World) -> Result<Value, String> {
    serde_json::to_value(Snapshot {
        version: 1,
        campaign: world.resource::<campaign::Campaign>().clone(),
        clock: world.resource::<GameClock>().clone(),
        paused: world.resource::<GamePaused>().0,
    })
    .map_err(|_| "Could not serialize the campaign.".to_string())
}

fn install(world: &mut World, value: Value) -> Result<(), String> {
    let snapshot: Snapshot = serde_json::from_value(value)
        .map_err(|_| "The saved campaign is invalid or uses a different game version.")?;
    let c = &snapshot.campaign;
    let n = c.actors.len();
    let p = c.economy.provinces.len();
    if snapshot.version != 1
        || !(1..=4).contains(&n)
        || !c.active
        || c.economy.players.len() != n
        || c.defeated.len() != n
        || c.governance.len() != n
        || c.profiles.len() != n
        || c.wars.len() != n
        || c.npc_wars.len() != n
        || c.invitations.len() != n
        || c.politics.len() != p
        || c.graph.len() != p
        || c.economy.adjacency.len() != p
        || c.military.provinces.len() != p
        || p != world.resource::<ProvinceOwnership>().province_count() + 1
        || snapshot.clock.month >= 12
        || !(-2..=2).contains(&snapshot.clock.speed_step)
        || !snapshot.clock.month_progress.is_finite()
        || snapshot.clock.month_progress < 0.
    {
        return Err(
            "The saved campaign has inconsistent players, provinces or clock values.".into()
        );
    }
    world.insert_resource(snapshot.campaign);
    world.insert_resource(snapshot.clock);
    world.resource_mut::<GamePaused>().0 = snapshot.paused;
    Ok(())
}

fn enter(
    world: &mut World,
    client: &mut OnlineClient,
    mut record: GameRecord,
) -> Result<(), String> {
    if record.members.is_empty()
        || record.members.len() > 4
        || !matches!(record.status.as_str(), "lobby" | "active" | "finished")
        || record.members.iter().any(|m| m.color >= PLAYER_COLORS.len() || m.player >= 4)
        || record.members.windows(2).any(|m| m[0].player >= m[1].player)
        || (record.status != "lobby"
            && record.members.iter().enumerate().any(|(i, m)| i != m.player))
        || record.state.as_ref().is_some_and(|s| {
            s["campaign"]["actors"].as_array().map(Vec::len) != Some(record.members.len())
        })
        || !record.members.iter().any(|m| m.player == record.player)
        || !crate::multiplayer::lobby::valid_recovery_code(&record.recovery_code)
    {
        return Err("The service returned an invalid player roster.".into());
    }
    if let Some(state) = record.state.take() {
        install(world, state.clone())?;
        client.base = Some(state);
        let campaign = world.resource::<campaign::Campaign>();
        let players = record
            .members
            .iter()
            .map(|m| PracticePlayer {
                name: m.display_name.clone(),
                color_index: m.color,
                rank: campaign.actors[m.player].rank.ladder_index(),
                main_province: campaign
                    .economy
                    .provinces
                    .iter()
                    .position(|p| p.owner == Some(m.player)),
            })
            .collect();
        let mut practice = world.resource_mut::<LocalPractice>();
        practice.players = players;
        practice.player_count = record.members.len();
        practice.active_player = record.player;
        practice.color_index = record.members[record.player].color;
        world
            .resource_mut::<ProvinceOwnership>()
            .start_game(&record.members.iter().map(|m| PLAYER_COLORS[m.color]).collect::<Vec<_>>());
        let mut resources = HudResources::default();
        resources.start_players(record.members.len(), world.resource::<ProvinceOwnership>());
        world.insert_resource(resources);
        world.insert_resource(ActiveGame::Online);
        if client.reconnect_lobby {
            let caller = record
                .members
                .iter()
                .find(|m| m.player == record.player)
                .expect("validated caller");
            let mut lobby = world.resource_mut::<LobbyPreview>();
            lobby.reset(&caller.display_name, record.code.clone());
            lobby.color_index = caller.color;
            world.resource_mut::<NextState<AppState>>().set(AppState::Lobby);
        } else {
            world.resource_scope(|world, mut loading: Mut<LoadingSequence>| {
                loading.begin(AppState::Map, &mut world.resource_mut::<NextState<AppState>>());
            });
        }
    } else {
        client.base = None;
        world.resource_mut::<LobbyPreview>().reset(
            &record
                .members
                .iter()
                .find(|m| m.player == record.player)
                .map(|m| m.display_name.clone())
                .unwrap_or_default(),
            record.code.clone(),
        );
        world.resource_mut::<LobbyPreview>().color_index =
            record.members.iter().find(|m| m.player == record.player).map(|m| m.color).unwrap_or(0);
        world.resource_mut::<NextState<AppState>>().set(AppState::Lobby);
    }
    client.record = Some(record);
    client.elapsed = 0.;
    client.connected = true;
    client.failures = 0;
    Ok(())
}

fn rpc(kind: Operation) -> &'static str {
    match kind {
        Operation::Create => "augustus_create",
        Operation::Join => "augustus_join",
        Operation::Open | Operation::LoadStarted => "augustus_record",
        Operation::Recover => "augustus_recover",
        Operation::Profile => "augustus_profile",
        Operation::Start => "augustus_start",
        Operation::Resume => "augustus_resume",
        Operation::List => "augustus_list",
        Operation::Save => "augustus_save",
        Operation::Sync => "augustus_sync",
        Operation::Leave => "augustus_leave",
    }
}

fn reconcile(
    world: &mut World,
    client: &mut OnlineClient,
    sync: SyncResponse,
) -> Result<(), String> {
    let resumed = sync.resume_generation > client.resume_generation;
    let record = client.record.as_mut().ok_or("No active game.")?;
    if sync.revision < record.revision {
        return Err("The service returned an older game revision.".into());
    }
    if let Some(members) = sync.members {
        record.members = members;
    }
    record.roster_token = sync.roster_token;
    record.status = sync.status;
    if record.status != "lobby" && client.base.is_none() {
        client.queue.push_back((
            Operation::LoadStarted,
            json!({"p_game_id":record.id,"p_connection":client.connection}),
        ));
        return Ok(());
    }
    if let Some(base) = &client.base {
        let mut current = snapshot(world)?;
        if record.player != 0 {
            current["clock"] = base["clock"].clone();
            current["paused"] = base["paused"].clone();
        }
        let from = client.reconcile_from.take().unwrap_or_else(|| base.clone());
        let local = patch::changes(&from, &current);
        let fallback = sync.state.is_some();
        let mut next = sync.state.unwrap_or_else(|| base.clone());
        let mut revision = record.revision;
        for event in sync.events {
            if event.revision != revision + 1 {
                return Err("The service skipped a game update.".into());
            }
            next = patch::apply(&next, &event.changes, false)?;
            revision = event.revision;
        }
        if !fallback && revision != sync.revision {
            return Err("The service skipped a game update.".into());
        }
        let merged = patch::apply(&next, &local, true);
        let merged = match merged {
            Ok(value) => value,
            Err(_) => {
                client.notice =
                    Some("Another player changed the game. Please retry your last action.".into());
                next.clone()
            },
        };
        let player = record.player;
        let known: std::collections::BTreeSet<_> = world
            .resource::<campaign::Campaign>()
            .notifications
            .history_for(player)
            .map(|n| n.id)
            .collect();
        install(world, merged)?;
        client.base = Some(next);
        let notices: Vec<_> = world
            .resource::<campaign::Campaign>()
            .notifications
            .history_for(player)
            .filter(|n| !known.contains(&n.id))
            .cloned()
            .collect();
        for notice in notices.into_iter().rev() {
            let mut toasts = world.resource_mut::<toasts::ToastQueue>();
            toasts.set_player(player);
            toasts.push(toasts::Toast::from_notice(notice));
        }
    }
    record.revision = sync.revision;
    client.resume_generation = client.resume_generation.max(sync.resume_generation);
    if resumed && client.reconnect_lobby {
        client.reconnect_lobby = false;
        world.resource_scope(|world, mut loading: Mut<LoadingSequence>| {
            loading.begin(AppState::Map, &mut world.resource_mut::<NextState<AppState>>());
        });
    }
    Ok(())
}

/// Pumps at most one HTTP operation per client, with state deltas and bounded backoff.
pub(super) fn update(world: &mut World) {
    world.resource_scope(|world,mut client:Mut<OnlineClient>| {
        client.elapsed+=world.resource::<Time>().delta_secs();
        client.since_sync+=world.resource::<Time>().delta_secs();
        let response=client.pending.as_ref().and_then(|p|p.receiver.lock().ok().and_then(|r|r.try_recv().ok()));
        if let Some(response)=response {
            let Some(mut pending)=client.pending.take() else {return;};
            if let Some(session)=response.session {client.session=Some(session);}
            let result=response.result.and_then(|value|match pending.kind {
                Operation::Create|Operation::Join|Operation::Open|Operation::LoadStarted|Operation::Recover|Operation::Start=> {
                    client.resume_generation=value.get("resume_generation").and_then(Value::as_u64).unwrap_or(0);
                    let record:GameRecord=serde_json::from_value(value).map_err(|_|"Invalid game response.")?;
                    client.reconnect_lobby=record.status=="active"&&matches!(pending.kind,Operation::Open|Operation::Recover)
                        &&client.record.as_ref().is_none_or(|r|r.status!="lobby");
                    enter(world,&mut client,record)
                }
                Operation::Profile=> {
                    let record:GameRecord=serde_json::from_value(value).map_err(|_|"Invalid player card response.")?;
                    enter(world,&mut client,record)
                }
                Operation::Resume=> {
                    client.resume_generation=value.get("resume_generation").and_then(Value::as_u64).ok_or("Invalid resume acknowledgement.")?;
                    client.reconnect_lobby=false;
                    world.resource_scope(|world,mut loading:Mut<LoadingSequence>| {
                        loading.begin(AppState::Map,&mut world.resource_mut::<NextState<AppState>>());
                    });Ok(())
                }
                Operation::List=> {client.games=serde_json::from_value(value).map_err(|_|"Invalid saved-game list.")?;Ok(())}
                Operation::Leave=> {
                    client.record=None;client.base=None;client.leave_requested=false;client.reconnect_lobby=false;
                    world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);Ok(())
                }
                Operation::Save=> {
                    let revision=value.get("revision").and_then(Value::as_u64).ok_or("Invalid save acknowledgement.")?;
                    let record=client.record.as_ref().ok_or("Missing game.")?;
                    if revision==record.revision+1 {
                        client.base=pending.sent.take();client.record.as_mut().ok_or("Missing game.")?.revision=revision;
                    } else if revision>record.revision+1 {
                        client.reconcile_from=pending.sent.take();
                        client.elapsed=30.;
                    } else {return Err("Invalid save revision.".into());}
                    client.notice=Some("Game saved.".into());client.save_requested=false;Ok(())
                }
                Operation::Sync=> {client.since_sync=0.;reconcile(world,&mut client,serde_json::from_value(value).map_err(|_|"Invalid synchronization response.")?)},
            });
            match result {
                Ok(())=> {
                    client.connected=true;client.failures=0;
                    // Background presence must not erase a foreground menu failure.
                    if pending.kind!=Operation::Sync || world.get_resource::<State<AppState>>().is_none_or(|state|
                        matches!(*state.get(),AppState::Map|AppState::GameMenu|AppState::GameSettings|AppState::EndGame)) {
                        client.error=None;
                    }
                }
                Err(error)=> {
                    client.connected=false;client.failures=client.failures.saturating_add(1);client.elapsed=0.;
                    if error.contains("State conflict") {
                        client.reconcile_from=pending.sent.take();client.elapsed=30.;client.report_error("Another player changed the game. Please retry your last action after syncing.");
                    } else {
                        // Replay exactly the same UUID/payload after an ambiguous network failure.
                        if error.starts_with("Connection failed") && matches!(pending.kind,Operation::Create|Operation::Save|Operation::Start|Operation::Resume) {
                            client.queue.push_front((pending.kind,pending.payload));
                            client.retry_sent=pending.sent.take();
                        }
                        if error.contains("unavailable")&&client.record.as_ref().is_some_and(|r|r.status=="lobby") {
                            client.record=None;client.base=None;world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
                        }
                        client.report_error(error);
                    }
                }
            }
        }
        if client.pending.is_some() {return;}
        if client.failures>0&&client.elapsed<(2_u32.saturating_pow(client.failures.min(5))) as f32 {return;}
        let queued=client.queue.pop_front();
        let mut sent=None;
        let request=if let Some(request)=queued {sent=client.retry_sent.take();Some(request)} else if let Some(record)=client.record.clone() {
            let in_game=matches!(*world.resource::<State<AppState>>().get(),AppState::Map|AppState::GameMenu|AppState::GameSettings|AppState::EndGame);
            let interval=if record.single_player {15.}else{2.};
            if client.elapsed<interval&&!client.save_requested&&!client.leave_requested {return;}
            if !record.single_player && client.since_sync>=6. && !client.leave_requested {
                Some((Operation::Sync,json!({"p_game_id":record.id,"p_connection":client.connection,"p_revision":record.revision,"p_roster_token":record.roster_token})))
            } else if record.status!="lobby"&&in_game&&record.status!="finished"&&client.reconcile_from.is_none() {
                if let Some(base)=&client.base {
                    let Ok(mut current)=snapshot(world) else {client.report_error("Could not save the campaign.");return;};
                    if record.player!=0 {current["clock"]=base["clock"].clone();current["paused"]=base["paused"].clone();}
                    // Fractional frame time alone never triggers cloud traffic.
                    let dirty=current["campaign"]!=base["campaign"]||current["paused"]!=base["paused"]
                        ||current["clock"]["speed_step"]!=base["clock"]["speed_step"];
                    if dirty||client.save_requested {
                        let changes=patch::changes(base,&current);
                        if !changes.is_empty() {
                            sent=Some(current);
                            Some((Operation::Save,json!({"p_game_id":record.id,"p_connection":client.connection,
                                "p_revision":record.revision,"p_request_id":supabase::request_id(),"p_changes":changes})))
                        } else {client.save_requested=false;None}
                    } else {None}
                } else {None}
            } else {None}
            .or_else(|| {
                if client.leave_requested && client.reconcile_from.is_none() {
                    Some((Operation::Leave,json!({"p_game_id":record.id,"p_connection":client.connection})))
                } else if record.single_player&&client.reconcile_from.is_none()&&client.elapsed<30. {None}
                else {Some((Operation::Sync,json!({"p_game_id":record.id,"p_connection":client.connection,"p_revision":record.revision,"p_roster_token":record.roster_token})))}
            })
        } else {None};
        if let Some((kind,payload))=request {
            if kind==Operation::Save&&sent.is_none() {sent=snapshot(world).ok();}
            let transport=match &client.transport {Ok(t)=>t,Err(e)=> {let error=e.clone();client.report_error(error);return;}};
            let receiver=transport.dispatch(client.session.clone(),rpc(kind).into(),payload.clone());
            client.pending=Some(Pending{kind,payload,receiver:Mutex::new(receiver),sent});
            client.elapsed=0.;
        }
    });
}

#[cfg(test)]
#[path = "../../tests/unit/online_campaign.rs"]
mod tests;

pub(super) fn draw_status(
    mut contexts: EguiContexts,
    client: Res<OnlineClient>,
    state: Res<State<AppState>>,
) {
    if !matches!(*state.get(), AppState::Map | AppState::EndGame) {
        return;
    }
    let Some(record) = &client.record else {
        return;
    };
    let message = if client.error.is_some() {
        None
    } else if record.members.iter().any(|m| m.player == 0 && !m.connected) {
        Some("Waiting for the host to reconnect. Game time is paused.")
    } else if record.members.iter().any(|m| !m.connected && !client.eliminated(m.player)) {
        Some("Waiting for players to reconnect. Game time is paused.")
    } else {
        client.notice.as_deref().filter(|message| !message.starts_with("Game saved"))
    };
    let Some(message) = message else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    egui::Area::new(egui::Id::new("augustus_online_status"))
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0., 55.))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            card_frame().show(ui, |ui| {
                ui.set_max_width(550.);
                ui.label(message);
            });
        });
}

pub(super) fn refresh_games(mut client: ResMut<OnlineClient>) {
    if !client.busy() {
        client.request(Operation::List, json!({}));
    }
}
