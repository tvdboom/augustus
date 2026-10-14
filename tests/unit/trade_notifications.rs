//! Trade popups must be retained, private, actionable, and concise.

use super::super::campaign_notifications::{NoticeAction, NoticeKind, NoticeSeverity};
use super::*;

fn campaign() -> Campaign {
    let count = 3;
    let mut provinces: Vec<_> = ["Italia", "Africa", "Britannia", "Achaia"]
        .into_iter()
        .map(|name| {
            EconomicProvince::new(
                name,
                60.0,
                Terrain::Farmland,
                true,
                [1.4, 0.8, 0.6],
                [8.0, 22.0, 33.0, 37.0],
                count,
            )
        })
        .collect();
    for (player, province) in provinces.iter_mut().take(count).enumerate() {
        province.owner = Some(player);
    }
    let adjacency = vec![vec![1, 2, 3], vec![0], vec![0], vec![0]];
    let mut campaign = Campaign {
        economy: EconomyWorld::new(count, provinces, adjacency.clone()),
        politics: vec![
            ProvincePolitics::owned(count, 0),
            ProvincePolitics::owned(count, 1),
            ProvincePolitics::owned(count, 2),
            ProvincePolitics::independent(count),
        ],
        actors: vec![PoliticalPlayer::default(); count],
        military: MilitaryWorld::new(4),
        graph: adjacency
            .into_iter()
            .map(|neighbors| MilitaryProvince {
                terrain: MilitaryTerrain::Plains,
                area: 100.0,
                road_level: 0,
                neighbors,
            })
            .collect(),
        wars: vec![vec![false; count]; count],
        npc_wars: vec![vec![false; 4]; count],
        invitations: vec![vec![false; count]; count],
        defeated: vec![false; count],
        ..Default::default()
    };
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.influence = 1000.0;
        wallet.resources = [1000.0; 3];
    }
    campaign.pull_wallets();
    campaign.economy.refresh_npc_markets(&campaign.inputs());
    campaign
}

fn offer(partner: TradeParty, frequency: TradeFrequency) -> TradeAgreement {
    TradeAgreement::new(
        TradeParty::Player(0),
        partner,
        TradeBundle {
            coin: 10.0,
            ..Default::default()
        },
        TradeBundle {
            resources: [0.0, 1.0, 0.0],
            ..Default::default()
        },
        frequency,
    )
}

#[test]
fn trade_offer_acceptance_and_closure_remain_in_both_participants_history() {
    let mut campaign = campaign();
    let id = campaign
        .propose_national_trade(offer(TradeParty::Player(1), TradeFrequency::Monthly))
        .unwrap();
    campaign.accept_national_trade(1, id).unwrap();
    campaign.end_trade(0, id, true).unwrap();
    let stopped = campaign.end_trade(0, id, false).unwrap();
    for player in 0..2 {
        let popups = campaign.notifications.drain_for(player);
        assert_eq!(popups.len(), 4);
        assert_eq!(
            popups.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(),
            ["Trade offer sent", "Trade accepted", "Trade closure scheduled", "Trade stopped",]
        );
        assert!(popups
            .iter()
            .all(|n| n.action == NoticeAction::OpenTrade(id) && !n.body.contains('#')));
        assert!(popups[3].body.contains(&stopped));
        assert_eq!(campaign.notifications.history_for(player).count(), 4);
        assert!(campaign.notifications.drain_for(player).is_empty());
    }
    assert_eq!(campaign.notifications.history_for(2).count(), 0);
    assert!(campaign.notifications.drain_for(2).is_empty());
    let saved = serde_json::to_string(&campaign).unwrap();
    let loaded: Campaign = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.notifications.history_for(0).count(), 4);
    assert!(loaded.notifications.history_for(0).next().unwrap().body.contains(&stopped));
}

#[test]
fn withdrawing_an_offer_and_completing_one_time_trades_are_retained_without_failed_commands() {
    let mut campaign = campaign();
    let id = campaign
        .propose_national_trade(offer(TradeParty::Player(1), TradeFrequency::OneTime))
        .unwrap();
    assert!(campaign.accept_national_trade(2, id).is_err());
    assert_eq!(campaign.notifications.history_for(0).count(), 1);
    campaign.end_trade(1, id, false).unwrap();
    for player in 0..2 {
        let notice = campaign.notifications.history_for(player).next().unwrap();
        assert_eq!(notice.title, "Trade offer cancelled");
        assert!(notice.body.ends_with("Trade offer cancelled."));
    }
    let id = campaign
        .propose_national_trade(offer(TradeParty::Player(1), TradeFrequency::OneTime))
        .unwrap();
    campaign.accept_national_trade(1, id).unwrap();
    for player in 0..2 {
        let notice = campaign.notifications.history_for(player).next().unwrap();
        assert_eq!(notice.title, "Trade completed");
        assert!(notice.body.ends_with("Exchange completed."));
    }
    let id = campaign
        .propose_national_trade(offer(TradeParty::Npc(3), TradeFrequency::OneTime))
        .unwrap();
    let notice = campaign.notifications.history_for(0).next().unwrap();
    assert_eq!(notice.title, "Trade completed");
    assert_eq!(notice.province, Some(3));
    assert_eq!(notice.body, "Achaia: Exchange completed.");
    assert_eq!(notice.action, NoticeAction::OpenTrade(id));
    assert_eq!(campaign.notifications.history_for(2).count(), 0);
}

#[test]
fn blocked_routes_immediately_record_a_brief_outcome_once_without_waiting_for_a_month() {
    let mut campaign = campaign();
    let id = campaign
        .propose_national_trade(offer(TradeParty::Npc(3), TradeFrequency::Monthly))
        .unwrap();
    campaign.npc_wars[0][3] = true;
    campaign.reconcile_provinces();
    assert_eq!(campaign.economy.month, 0);
    let trade = campaign.economy.trades.iter().find(|t| t.id == id).unwrap();
    assert_eq!(trade.status, TradeStatus::Suspended);
    assert!(trade.last_failure.is_some());
    let notice = campaign.notifications.history_for(0).next().unwrap();
    assert_eq!(notice.kind, NoticeKind::TradeInterrupted);
    assert_eq!(notice.severity, NoticeSeverity::Warning);
    assert_eq!(notice.province, Some(3));
    assert_eq!(notice.action, NoticeAction::OpenTrade(id));
    assert_eq!(notice.body, "Achaia: Trade route suspended.");
    let total = campaign.notifications.history_for(0).count();
    campaign.reconcile_provinces();
    assert_eq!(campaign.notifications.history_for(0).count(), total);
    assert_eq!(campaign.notifications.history_for(1).count(), 0);
}

#[test]
fn npc_resource_changes_do_not_interrupt_accepted_routes_or_create_warnings() {
    let mut campaign = campaign();
    let id = campaign
        .propose_national_trade(offer(TradeParty::Npc(3), TradeFrequency::Monthly))
        .unwrap();
    campaign.economy.provinces[3].potential[1] = 0.0;
    for month in 1..=6 {
        let report = campaign.economy.advance_month(&campaign.inputs());
        campaign.record_economic_events(&report);
        let trade = campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap();
        assert_eq!(trade.status, TradeStatus::Active);
        assert_eq!(trade.last_executed_month, Some(month));
        assert_eq!(trade.last_fulfillment, 1.0);
        assert_eq!(campaign.economy.provinces[3].market.resources[1].export_capacity, 0.0);
        assert_eq!(
            campaign
                .notifications
                .history_for(0)
                .filter(|notice| notice.kind == NoticeKind::TradeInterrupted)
                .count(),
            0
        );
    }
}

#[test]
fn monthly_failures_keep_each_agreement_and_brief_cancellation_outcome() {
    let mut campaign = campaign();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let id = campaign
            .propose_national_trade(offer(TradeParty::Player(1), TradeFrequency::Monthly))
            .unwrap();
        campaign.accept_national_trade(1, id).unwrap();
        ids.push(id);
    }
    campaign.economy.players[0].coin = 0.0;
    let mut report = campaign.economy.begin_month(&campaign.inputs());
    assert_eq!(report.events.len(), 2);
    campaign.record_economic_events(&report);
    campaign.record_economic_events(&report);
    for player in 0..2 {
        let notices: Vec<_> = campaign
            .notifications
            .history_for(player)
            .filter(|n| n.kind == NoticeKind::TradeInterrupted)
            .collect();
        assert_eq!(notices.len(), 2, "Same-month failures must not overwrite each other");
        for id in &ids {
            let reason = report
                .events
                .iter()
                .find_map(|event| match event {
                    EconomyEvent::TradeSuspended {
                        agreement,
                        reason,
                    } if agreement == id => Some(reason),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                campaign
                    .economy
                    .trades
                    .iter()
                    .find(|trade| trade.id == *id)
                    .unwrap()
                    .last_failure
                    .as_ref(),
                Some(reason)
            );
            assert!(notices.iter().any(|n| n.action == NoticeAction::OpenTrade(*id)
                && n.body.ends_with("Trade route suspended.")));
        }
    }
    let limit = campaign.economy.config.trade.failure_limit;
    for trade in &mut campaign.economy.trades {
        trade.failure_months = limit - 1;
    }
    report = campaign.economy.begin_month(&campaign.inputs());
    campaign.record_economic_events(&report);
    for player in 0..2 {
        let notice = campaign.notifications.history_for(player).next().unwrap();
        assert_eq!(notice.title, "Trade cancelled");
        assert!(notice.body.ends_with("Trade route cancelled."));
    }
    assert_eq!(campaign.notifications.history_for(2).count(), 0);
}

#[test]
fn completed_cancellation_notice_reaches_both_human_participants_and_npc_history() {
    for partner in [TradeParty::Player(1), TradeParty::Npc(3)] {
        for overdue in [false, true] {
            let mut campaign = campaign();
            let id =
                campaign.propose_national_trade(offer(partner, TradeFrequency::Monthly)).unwrap();
            if matches!(partner, TradeParty::Player(_)) {
                campaign.accept_national_trade(1, id).unwrap();
            }
            campaign.end_trade(0, id, true).unwrap();
            let due = campaign.economy.trades[0].cancellation_month.unwrap();
            campaign.economy.month = if overdue {
                due
            } else {
                due - 1
            };
            let report = campaign.economy.begin_month(&campaign.inputs());
            assert!(report.events.contains(&EconomyEvent::TradeNoticeCompleted {
                agreement: id
            }));
            campaign.record_economic_events(&report);
            for player in 0..if matches!(partner, TradeParty::Player(_)) {
                2
            } else {
                1
            } {
                let notice = campaign.notifications.history_for(player).next().unwrap();
                assert_eq!(notice.title, "Trade route ended");
                assert_eq!(notice.severity, NoticeSeverity::Info);
                assert_eq!(notice.month, report.month);
                assert!(notice.body.ends_with("Trade route closed."));
                assert_eq!(notice.action, NoticeAction::OpenTrade(id));
            }
            assert_eq!(campaign.notifications.history_for(2).count(), 0);
        }
    }
}

#[test]
fn trade_notifications_appear_on_affected_provinces_without_leaking_to_other_players() {
    use crate::app::ui::campaign_notices::belongs_to_province;
    let mut campaign = campaign();
    let npc = campaign
        .propose_national_trade(offer(TradeParty::Npc(3), TradeFrequency::Monthly))
        .unwrap();
    let human = campaign
        .propose_national_trade(offer(TradeParty::Player(1), TradeFrequency::Monthly))
        .unwrap();
    campaign.accept_national_trade(1, human).unwrap();
    campaign.notify_market_trade(0, MarketSide::Buy);
    let on_province = |player, province| {
        campaign
            .notifications
            .history_for(player)
            .filter(|notice| belongs_to_province(&campaign, notice, province))
            .map(|notice| notice.action)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        on_province(0, 0),
        [
            NoticeAction::OpenMarket,
            NoticeAction::OpenTrade(human),
            NoticeAction::OpenTrade(human),
            NoticeAction::OpenTrade(npc)
        ]
    );
    assert_eq!(on_province(0, 1), [NoticeAction::OpenTrade(human), NoticeAction::OpenTrade(human)]);
    assert_eq!(on_province(0, 3), [NoticeAction::OpenTrade(npc)]);
    assert_eq!(on_province(1, 0), [NoticeAction::OpenTrade(human), NoticeAction::OpenTrade(human)]);
    assert!(on_province(0, 2).is_empty());
    assert!(on_province(1, 3).is_empty());
    assert!(on_province(2, 0).is_empty());
    assert!(on_province(0, 999).is_empty());
}
