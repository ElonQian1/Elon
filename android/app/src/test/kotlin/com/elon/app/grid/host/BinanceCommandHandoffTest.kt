package com.elon.app.grid.host

import android.app.Application
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.LooperMode

@RunWith(RobolectricTestRunner::class)
@Config(sdk=[34],manifest=Config.NONE,application=Application::class)
@LooperMode(LooperMode.Mode.PAUSED)
class BinanceCommandHandoffTest {
    @Test fun aRebuiltPageCanOpenAfterTheOriginalSubscriberLeaves() {
        for((kind,version) in listOf("create" to 2,"manage" to 2,"manage" to 3,"manage" to 4))BinanceCommandFixture(kind,version).use {f->
            f.subscribe(f.old);f.call("open");f.subscribe(f.next)
            assertEquals("busy",f.call("open",f.next)["status"])
            f.unsubscribe(f.old)
            assertNotEquals("busy",f.call("open",f.next)["status"])
            assertEquals(f.next,f.field("operation"))
            assertThrows(IllegalArgumentException::class.java){f.call("close",f.old)}
            assertEquals(f.next,f.field("operation"))
        }
    }
    @Test fun removingAnOldSubscriberNotifiesTheNewPageToTryAgain() {
        BinanceCommandFixture("create").use {f->
            f.subscribe(f.old);f.call("open");f.subscribe(f.next);f.idle();f.topics.clear()
            f.unsubscribe(f.old);f.idle()
            assertEquals(listOf("state"),f.topics)
        }
    }
    @Test fun noNewSubscriberOrWrongSubscriptionKindCannotTakeOwnership() {
        for(kind in listOf("create","manage"))BinanceCommandFixture(kind).use {f->
            f.subscribe(f.old);f.call("open");f.unsubscribe(f.old)
            assertEquals("busy",f.call("open",f.next)["status"])
            f.subscribe(f.next,"read")
            assertEquals("busy",f.call("open",f.next)["status"])
            assertEquals(f.old,f.field("operation"))
        }
    }
    @Test fun onlyOpenCanAdoptAndOldCommandsCannotClearTheNewOwner() {
        for(kind in listOf("create","manage"))BinanceCommandFixture(kind).use {f->
            f.subscribe(f.old);f.call("open");f.unsubscribe(f.old);f.subscribe(f.next)
            assertThrows(IllegalArgumentException::class.java){f.call("poll",f.next)}
            assertEquals(f.old,f.field("operation"));f.call("open",f.next)
            for(action in listOf("poll","cancel","close"))assertThrows(IllegalArgumentException::class.java){f.call(action,f.old)}
            assertEquals(f.next,f.field("operation"));assertNull(f.host.view)
        }
    }
    @Test fun preparedCreateIntentAndBothOldAndNewPermitsAreInvalidated() {
        BinanceCommandFixture("create").use {f->
            f.subscribe(f.old);f.call("open");val attempt=f.seedCreate("prepared");f.seedPermit();assertTrue(f.permitValid(f.old))
            f.unsubscribe(f.old);f.subscribe(f.next);f.call("open",f.next)
            assertEquals("idle",attempt.status);assertFalse(f.permitValid(f.old));assertFalse(f.permitValid(f.next))
            assertEquals("",f.field("preparation"));assertEquals("",f.field("digest"));assertNull(f.journal.read())
        }
    }
    @Test fun preparedManageIntentAndItsSelectedTargetAreNotTransferredAsAuthority() {
        BinanceCommandFixture("manage").use {f->
            f.subscribe(f.old);f.call("open");val state=f.seedManage("prepared");f.seedPermit();f.set("selected","123")
            f.unsubscribe(f.old);f.subscribe(f.next);f.call("open",f.next)
            assertEquals("idle",state.status);assertEquals("",f.field("selected"))
            assertFalse(f.permitValid(f.old));assertFalse(f.permitValid(f.next));assertNull(f.journal.read())
        }
    }
    @Test fun everyUnresolvedCreateOutcomeKeepsItsSessionJournalAndAttempt() {
        for(status in listOf("submitting","unknown","accepted","observed"))BinanceCommandFixture("create").use {f->
            f.subscribe(f.old);f.call("open");val attempt=f.seedCreate(status);val raw=f.journal.read()
            val session=f.field("session");val observer=f.host.onCreateObservation;f.seedPermit();f.setBusy(status=="submitting")
            f.unsubscribe(f.old);f.subscribe(f.next);f.call("open",f.next)
            assertSame(attempt,f.field("attempt"));assertSame(session,f.field("session"));assertSame(observer,f.host.onCreateObservation)
            assertEquals(status,attempt.status);assertEquals(raw,f.journal.read());assertEquals(status=="submitting",f.busy())
            assertFalse(f.permitValid(f.old));assertFalse(f.permitValid(f.next));assertEquals(f.next,f.field("operation"));assertNull(f.host.view)
        }
    }
    @Test fun everyUnresolvedManageOutcomeKeepsItsSessionJournalAndTarget() {
        for(version in 2..4)for(status in listOf("submitting","unknown","accepted","observed"))BinanceCommandFixture("manage",version).use {f->
            f.subscribe(f.old);f.call("open");val state=f.seedManage(status);val raw=f.journal.read()
            val session=f.field("session");val observer=f.host.onCreateObservation;f.seedPermit();f.setBusy(status=="submitting")
            f.unsubscribe(f.old);f.subscribe(f.next);f.call("open",f.next)
            assertSame(state,f.field("state"));assertSame(session,f.field("session"));assertSame(observer,f.host.onCreateObservation)
            assertEquals(status,state.status);assertEquals("123",state.id);assertEquals(raw,f.journal.read());assertEquals(status=="submitting",f.busy())
            assertFalse(f.permitValid(f.old));assertFalse(f.permitValid(f.next));assertEquals(f.next,f.field("operation"));assertNull(f.host.view)
        }
    }
    @Test fun aDifferentManageProtocolStillRequiresExplicitSessionClosure() {
        BinanceCommandFixture("manage",2).use {f->
            f.subscribe(f.old);f.call("open");f.unsubscribe(f.old);f.subscribe(f.next)
            assertEquals("busy",f.call("open",f.next,4)["status"]);assertEquals(f.old,f.field("operation"))
        }
    }
    @Test fun repeatedPageRebuildsPreserveTheOriginalInFlightCreateAttempt() {
        BinanceCommandFixture("create").use {f->
            f.subscribe(f.old);f.call("open");val attempt=f.seedCreate("submitting");val raw=f.journal.read();val session=f.field("session")
            var previous=f.old
            for(id in listOf(f.next,"d".repeat(64),"e".repeat(64))) {
                f.unsubscribe(previous);f.subscribe(id);f.call("open",id)
                assertThrows(IllegalArgumentException::class.java){f.call("close",previous)}
                assertSame(attempt,f.field("attempt"));assertSame(session,f.field("session"));assertEquals(raw,f.journal.read())
                assertEquals("submitting",attempt.status);assertEquals(id,f.field("operation"));previous=id
            }
        }
    }
}
