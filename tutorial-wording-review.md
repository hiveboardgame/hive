# Tutorial wording review

Branch: improve-tutorial-content
Baseline commit: 8efd31e3

All 42 released lessons were reviewed: descriptions, instructions, wrong-move messages, conditional feedback, completion text and tips.
Positions, move rules, completion conditions, lesson order and hint triggers are unchanged.

Lesson text: 5,098 words before; 3,300 after (35.3% shorter), excluding piece-icon tokens.
Every instruction box is under 32 words. Actual line counts depend on window width and inline icons.
An empty line separates the explanation from the final action wherever helpful. Intentional line breaks are now preserved in descriptions.
[[wQ]], [[bP]], and similar tokens render as the corresponding piece icons in the app.

Terminology: move for a Pillbug action with a specified destination; lift for its general ability or an unrestricted destination.

Ask to revise or restore any item by lesson number and text label. All changed blocks below include the exact old and new wording.

Upstream check: this branch is missing 30 upstream/main commits and none from upstream/tutorial. Upstream/main already says Create new game. No merge was performed.

## 1. Welcome to Hivegame.com!

Introduced reserve in plain language; deferred the connected-hive rule until lesson 5.

Lesson ID: basics.welcome
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> Hive has no board: the pieces are the board. Every move must still keep the hive connected, and the winner is the player who surrounds the opponent's Queen Bee [[bQ]].

After:

> Hive has no board: the pieces form the playing area.
> Surround your opponent's Queen Bee [[bQ]] to win.

### Description 2

Before:

> When the game starts, all pieces are in the reserve. White goes first.

After:

> Your unplayed pieces are your **reserve**.
> White places first, then Black.
> Players take turns, playing one piece each turn.

### Instruction 1

Before:

> This is how a game starts. You play as White. Place any piece from your reserve, except the Queen Bee [[wQ]].
> Click on the piece you choose in the reserve, then click on the highlighted hex.

After:

> You play as White. Start with any piece except the Queen Bee [[wQ]].
>
> Click a reserve piece, then the highlighted hex.

### Wrong-move message 1

Before:

> Click a piece in your reserve, then the highlighted hex. The Queen may not open the game.

After:

> Choose any reserve piece except the Queen Bee, then click the highlighted hex.

### Instruction 2

Before:

> Now play as Black. Place any piece from Black's reserve, except the Queen Bee [[bQ]], next to White's first piece.

After:

> Now play as Black. The Queen Bee [[bQ]] cannot start.
>
> Place a reserve piece next to White's piece.

### Wrong-move message 2

Before:

> Choose a Black piece other than the Queen Bee, then place it next to White's piece.

After:

> Place any Black reserve piece except the Queen Bee next to White's piece.

### Instruction 3

Before:

> Well done! You have placed the first piece for both White and Black.

After:

> Well done! Both players have placed their first piece.

## 2. Placing new pieces

Removed the early Ant recommendation; kept placement and movement rules distinct.

Lesson ID: basics.placing
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> The first two pieces of the game have to touch each other.

After:

> The first two pieces must touch.

### Description 2

Before:

> From now on, **every new piece you place must touch at least one of your own pieces and none of your opponent's pieces**.

After:

> After that, **new pieces must touch your own pieces and none of your opponent's**.

### Description 3

Before:

> Here both players started with their Ladybug [[wL]], a popular opening. Ants [[wA1]] are a good piece to place next since they are very flexible, but there are plenty of interesting setups out there!

After:

> Both players opened with a Ladybug [[wL]] [[bL]], a common start.

### Instruction 1

Before:

> Play as White: pick any piece you like from the reserve and place it. Remember:
>
> - New pieces entering the game cannot touch opponent pieces.
>
> The board will show the hexes available for placement.

After:

> You play as White. New pieces cannot touch Black's pieces.
>
> Choose any reserve piece, then a highlighted hex.

### Wrong-move message 1

Before:

> Choose a piece from your reserve, then a highlighted hex.

After:

> Choose any reserve piece, then a highlighted hex.

### Instruction 2

Before:

> Well done! None of the hexes touching a black piece were available to your piece. Remember this rule only applies to placing. Once a piece is in play, it may move also to hexes touching the opponent pieces.
>
> Black replied to your move by placing their Queen Bee [[bQ]].

After:

> Well done! This placement rule does not apply to movement.
> Black replied by placing the Queen Bee [[bQ]].

## 3. An army needs its Queen

Reduced repeated timing instructions; retained the first-piece restriction.

Lesson ID: basics.queen_rule
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> Until your Queen [[wQ]] is on the board, none of your pieces may move: you can only place new ones.

After:

> You cannot move pieces until your Queen Bee [[wQ]] is in play.

### Description 2

Before:

> She cannot open the game, and she must arrive by your 4th turn.

After:

> She cannot be your first piece and must be placed by your 4th turn.

### Instruction 1

Before:

> The Queen Bee [[wQ]] has to be placed by your 4th turn. Put her in play now!

After:

> This is your 4th turn.
>
> Place your Queen Bee [[wQ]].

### Wrong-move message 1

Before:

> On your fourth turn, only your Queen may be placed.

After:

> Place your Queen Bee. She must enter by your 4th turn.

### Instruction 2

Before:

> Once your Queen Bee [[wQ]] is in play, you can move your pieces already in play during your turn.

After:

> Your Queen Bee [[wQ]] is in play!
> You may now move pieces already on the hive.

### Tip 2

Before:

> Tip: Bring your Queen Bee [[wQ]] into play as soon as possible so you can move your pieces early.

After:

> Tip: Place your Queen Bee [[wQ]] early to make movement available.

## 4. Move your pieces

Kept the explanation that a placement is legal but does not solve this exercise.

Lesson ID: basics.move_pieces
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> Your Queen Bee [[wQ]] is now in play, so you can move pieces that are already on the hive.

After:

> With your Queen Bee [[wQ]] in play, your pieces can move.

### Description 2

Before:

> On each turn, you may move a piece in play or place a new piece from your reserve.

After:

> Each turn, either move a piece on the hive or place a reserve piece.

### Instruction 1

Before:

> You play as White. Click your pieces to see their legal moves, then move any piece already in play.

After:

> You play as White.
>
> Click a piece on the hive to see its moves, then move it.

### Wrong-move message 1

Before:

> That placement is allowed in a game, but this lesson is about moving pieces already in play. Move one of your pieces on the board.

After:

> That placement is allowed in a game. In this lesson, move a piece already on the hive.

### Instruction 2

Before:

> Well done! You have moved a piece in play.

After:

> Well done! You moved a piece already in play.

## 5. One Hive Rule

Defined pinned once; removed repeated strategy advice and reassurance.

Lesson ID: basics.one_hive
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> This is the most important rule in Hive: **all pieces in play must form one connected group at all times**.

After:

> The **One Hive Rule**: all pieces must stay connected throughout a move.

### Description 2

Before:

> A piece cannot move if moving it would break the hive into two or more groups. Such a piece is **pinned**.

After:

> A piece is **pinned** if removing it would split the hive into separate groups.

### Description 3

Before:

> To check whether a move is legal, imagine taking the piece you want to move into your hand. If the rest of the hive breaks into two or more disconnected groups, that move is illegal, even if the piece could land somewhere that would reconnect the hive.

After:

> Imagine picking the piece up.
> If the remaining pieces become disconnected, it cannot move—even if its destination would reconnect them.

### Instruction 1

Before:

> The highlighted pieces cannot move without breaking the hive. These pieces are pinned.
>
> You play as White. Your Queen Bee [[wQ]], Ant [[wA1]] and Mosquito [[wM]] can move!
>
> Click your pieces to inspect their moves, then move one of them. Don't worry if you don't know how pieces move yet, you'll learn soon!

After:

> You play as White. Highlighted pieces are pinned.
>
> Click your Queen Bee [[wQ]], Ant [[wA1]] or Mosquito [[wM]], then move one.

### Wrong-move message 1

Before:

> Move a piece that's already on the board instead of placing a new one.

After:

> Move a piece already on the hive; do not place a reserve piece.

### Instruction 2

Before:

> Now play as Black. Any highlighted pieces cannot move in this position. Check your pieces, then make any legal move with a Black piece already in play.

After:

> Now play as Black. Highlighted pieces cannot move.
>
> Move any Black piece already on the hive.

### Wrong-move message 2

Before:

> Move a Black piece that's already on the board instead of placing a new one.

After:

> Move a Black piece already on the hive.

### Instruction 3

Before:

> Well done! You have kept the hive connected while moving both colours.

After:

> Well done! Both moves kept the hive connected.

### Win message 3

Before:

> Good job, Black won!

After:

> Black wins!

### Closing description

Before:

> A piece that holds the hive together is pinned. Pinning enemy pieces is one of the most important ideas in Hive, and it comes straight out of this rule: every move must preserve one connected hive.

After:

_Removed._

## 6. How to Win

Removed real-game background; kept the colour-independent win rule.

Lesson ID: basics.surround
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> The goal in Hive is to surround the opponent's Queen Bee [[bQ]] on all six sides. Both your pieces and your opponent's pieces count.

After:

> Surround the opponent's Queen Bee [[bQ]] on all six sides to win.
> Pieces of either colour count.

### Description 2

Before:

> This is the end of a real game between two strong players. The black Queen [[bQ]] has pieces on five of her six sides, and it's White's move.

After:

> The Black Queen [[bQ]] has just one empty neighbouring hex.

### Instruction 1

Before:

> Move one of your Ants [[wA1]] into the highlighted hex to completely surround the black Queen Bee [[bQ]].

After:

> You play as White.
>
> Move an Ant [[wA1]] into the green hex to surround the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Move one of your Ants into the highlighted hex.

After:

> Move a White Ant into the green hex.

### Instruction 2

Before:

> That's the whole goal of Hive: surround your opponent's Queen Bee [[bQ]] first.
>
> It doesn't matter what colour the pieces surrounding the Queen Bee [[wQ]] are. Here, the black Ladybug [[bL]] is actually hurting Black!

After:

> Well done! Even Black's Ladybug [[bL]] helped surround the Black Queen [[bQ]].
> The surrounding pieces can be either colour.

### Tip 2

Before:

> Tip: Be careful which pieces you place next to your own Queen Bee [[wQ]]!

After:

> Tip: Be careful what you place beside your own Queen Bee [[wQ]].

## 7. Draw?

Removed the unnecessary claim about what the opponent would have forced.

Lesson ID: basics.draw
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> Games can also end in a draw. The most common way this happens is when both Queen Bees [[wQ]] [[bQ]] are surrounded at the same time.

After:

> If both Queen Bees [[wQ]] [[bQ]] become surrounded at the same time, the game is a draw.
> A draw has no winner.

### Description 2

Before:

> In this game, the highlighted hex is crucial: by moving a piece there, both Queen Bees [[wQ]] [[bQ]] lose their last hex of freedom!

After:

> The green hex is the last empty space beside both Queen Bees [[wQ]] [[bQ]].

### Instruction 1

Before:

> You play as White. Move your Grasshopper [[wG1]] to surround both Queen Bees [[wQ]] [[bQ]] at the same time.

After:

> You play as White.
>
> Jump your Grasshopper [[wG1]] into the green hex to surround both Queen Bees [[wQ]] [[bQ]].

### Wrong-move message 1

Before:

> Move the white Grasshopper into the hole between the two Queen Bees.

After:

> Move the White Grasshopper into the hole between the Queen Bees.

### Instruction 2

Before:

> Well done! If you hadn't done it, your opponent would have forced the draw.

After:

> Both Queen Bees [[wQ]] [[bQ]] are surrounded. It's a draw!

### Tip 2

Before:

> Tip: When the Queen Bees [[wQ]] [[bQ]] are very close to each other, a draw is more likely!

After:

> Tip: Nearby Queen Bees [[wQ]] [[bQ]] can become surrounded together.

## 8. Can I pass?

Removed blame and repeated pass rules; preserved the two-placement objective.

Lesson ID: basics.pass
Source: apis/src/tutorial/basics.rs

### Description 1

Before:

> You may pass only when you have no legal move and no legal placement. If you can play, you must take your turn.

After:

> You **must pass** if you have no legal move or placement.
> Otherwise, you must play.

### Description 2

Before:

> Black has spent too many turns moving only a few pieces. White has blocked every place where a new Black piece could enter, and none of Black's pieces can move. This is called a **shut-out**.

After:

> Black's pieces cannot move, and White has blocked every Black placement.
> This is a **shut-out**.

### Description 3

Before:

> Black must pass, so White plays again. Black will keep passing only while it has no legal move or placement; a pass is never a choice when a move is available.

After:

> Black must pass, so White plays again.

### Instruction 1

Before:

> Black has no legal move or placement and has passed. You play as White: place any piece from your reserve.

After:

> Black has passed. You play as White again.
>
> Place any reserve piece.

### Wrong-move message 1

Before:

> Place a piece from your reserve so Black stays shut out.

After:

> Place a reserve piece so Black stays shut out.

### Instruction 2

Before:

> Black still has no legal move or placement and passed again. You play as White once more: place another piece from your reserve.

After:

> Black must pass again.
>
> Place another White reserve piece.

### Wrong-move message 2

Before:

> Place another piece from your reserve.

After:

> Place another White reserve piece.

### Instruction 3

Before:

> Well done! Black had to pass twice, so you placed two pieces in a row.

After:

> Well done! Black passed twice, so White placed two pieces in a row.

### Closing description

Before:

> Black can play again as soon as a legal move or placement becomes available. Remember: you must play if you have a legal move, even when it leads to a loss.

After:

> Black can play again as soon as a legal move or placement becomes available.

## 9. Queen Bee

Removed the chess comparison and repeated two-move command.

Lesson ID: bugs.queen
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Your Queen [[wQ]] is your most important piece: if she gets surrounded, you lose the game. Think of her like the king in chess.

After:

> Keep your Queen [[wQ]] safe: if she is surrounded, you lose.

### Description 2

Before:

> She moves by **crawling** one hex per turn. Many pieces in Hive move by crawling.

After:

> She moves one hex per turn by crawling.

### Description 3

Before:

> **Crawling** simply means sliding along the edge of the hive.

After:

> **Crawling** means sliding along the hive's edge.

### Instruction 1

Before:

> You play as White. Make your Queen [[wQ]] reach the green hex in 2 moves to complete the lesson. Click on your pieces to see where they can move.

After:

> You play as White.
>
> Move your Queen [[wQ]] to the green hex in two moves.

### Wrong-move message 1

Before:

> Aim to reach the green hex as quickly as possible.

After:

> Choose the shortest route to the green hex.

### Instruction 2

Before:

> You play as White. Make your Queen [[wQ]] reach the green hex in 2 moves to complete the lesson. Click on your pieces to see where they can move.

After:

> Your turn again.
>
> Move your Queen [[wQ]] into the green hex.

### Wrong-move message 2

Before:

> Aim to reach the green hex as quickly as possible.

After:

> Move your Queen into the green hex.

### Instruction 3

Before:

> Well done! The Queen [[wQ]] has reached her destination and pinned the two Ants [[bA1]] [[bA2]] in the process.

After:

> Well done! Your Queen [[wQ]] reached the goal and pinned both Black Ants [[bA1]] [[bA2]].

### Tip 3

Before:

> Tip: The Queen [[wQ]] doesn't need to move often, but take your chances to escape from your opponent's pieces if you can!

After:

> Tip: Move your Queen [[wQ]] out of danger when you can.

## 10. Freedom to Move Rule

Explained gate here instead of referring to a later lesson; made feedback specific to each step.

Lesson ID: bugs.freedom_to_move
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Crawling is only possible if pieces can slide along the edges of the hive. Here, the White Ladybug [[wL]] and Ant [[wA2]] obstruct the movement of the Queen [[wQ]] according to the Freedom to Move Rule.

After:

> Crawling pieces must have room to slide.
> Your Ladybug [[wL]] and Ant [[wA2]] block the Queen's [[wQ]] route.

### Description 2

Before:

> These formations are also called Gates (you will learn more about them later!).

After:

> This narrow opening is a **gate**: the Queen [[wQ]] cannot squeeze through it.

### Instruction 1

Before:

> You play as White. Move your Ant [[wA2]] to allow your Queen [[wQ]] to reach the green hex.

After:

> You play as White.
>
> Move your Ant [[wA2]] to open a path for the Queen [[wQ]].

### Wrong-move message 1

Before:

> Aim to reach the green hex as quickly as possible.

After:

> Move your Ant away from the opening, not into the green hex.

### Instruction 2

Before:

> You play as White. Move your Queen [[wQ]] into the green hex.

After:

> The path is open.
>
> Move your Queen [[wQ]] into the green hex.

### Wrong-move message 2

Before:

> Aim to reach the green hex as quickly as possible.

After:

> Move your Queen into the green hex.

### Instruction 3

Before:

> Well done! Your Ant [[wA2]] opened the Gate, allowing your Queen [[wQ]] to crawl through.

After:

> Well done! Moving your Ant [[wA2]] let the Queen [[wQ]] slide through.

## 11. Don't break the hive

Shortened the fixed-tile explanation and removed repeated completion wording.

Lesson ID: bugs.dont_break_the_hive
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> You are not allowed to break the hive during a move, even if it would reconnect at the end of the move.

After:

> A piece must stay connected to the hive throughout its move.

### Description 2

Before:

> Only your White Queen [[wQ]] can move here. The blank White and Black tiles stay fixed and form the hive.

After:

> Only your Queen [[wQ]] can move here.
> The blank tiles stay fixed.

### Description 3

Before:

> Keep your Queen [[wQ]] touching the hive as she crawls around it. Reach the green hex to completely surround the Black Queen [[bQ]].

After:

> Go around the hive to reach the last empty space beside the Black Queen [[bQ]].

### Instruction 1

Before:

> Blank tiles cannot move. You can keep moving until the lesson is completed. You play as White. Move your Queen [[wQ]] around the hive to reach the green hex and surround the Black Queen [[bQ]].

After:

> You play as White. Keep moving your Queen [[wQ]].
>
> Reach the green hex to surround the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Keep your Queen touching the hive and aim for the green hex.

After:

> Keep your Queen touching the hive as she moves toward the green hex.

## 12. Ant (first moves)

Replaced vague attack language with the exact adjacency objective.

Lesson ID: bugs.ant_first_moves
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Ant [[wA1]] can crawl around the edge of the hive, as many spaces as you want in a single turn.

After:

> The Ant [[wA1]] crawls any number of hexes around the hive's edge in one turn.

### Instruction 1

Before:

> You play as White. Move your Ant [[wA1]] to any space next to the Black Queen [[bQ]].

After:

> You play as White.
>
> Move your Ant [[wA1]] beside the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Move your Ant next to the Black Queen.

After:

> Move your Ant into any hex touching the Black Queen.

### Instruction 2

Before:

> Well done! Your Ant [[wA1]] is attacking the Black Queen [[bQ]].

After:

> Well done! Your Ant [[wA1]] is beside the Black Queen [[bQ]].

## 13. Gate

Kept exploration possible without spending instruction-box space on it.

Lesson ID: bugs.ant
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> A Gate can stop a crawling piece from sliding into an empty hex.

After:

> A gate can block a crawling piece.

### Description 2

Before:

> Here, your Queen [[wQ]] forms one side of the Gate. Move her away, then slide your Ant [[wA1]] into the last space around the Black Queen [[bQ]]. Blank tiles stay fixed.

After:

> Your Queen [[wQ]] forms one side of this gate.
> Move her away so your Ant [[wA1]] can enter the hole. Blank tiles stay fixed.

### Instruction 1

Before:

> You play as White. Surround the Black Queen [[bQ]] using your Ant [[wA1]], removing the Gate first. You may move your Ant [[wA1]] around to explore.

After:

> You play as White. Open the gate.
>
> Use your Ant [[wA1]] to surround the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Open the Gate with your Queen, then move your Ant into the hole.

After:

> Move your Queen away, then slide your Ant into the hole.

### Instruction 2

Before:

> Well done! Your Ant [[wA1]] passed through the open Gate and surrounded the Black Queen [[bQ]].

After:

> Well done! Your Ant [[wA1]] passed through and surrounded the Black Queen [[bQ]].

## 14. Ant (attack!)

Used the already-defined word pinned and removed duplicate explanation.

Lesson ID: bugs.ant_attack
Source: apis/src/tutorial/bugs.rs

### Instruction 1

Before:

> Let's use our Ant [[wA1]] to start an attack. Move your Ant [[wA1]] near the opponent's Queen [[bQ]] and make sure she can't move.

After:

> You play as White.
>
> Move your Ant [[wA1]] beside the Black Queen [[bQ]] and pin her.

### Wrong-move message 1

Before:

> Move your Ant next to the Black Queen so she cannot move without breaking the hive.

After:

> Move your Ant beside the Black Queen where she cannot leave without splitting the hive.

### Instruction 2

Before:

> Well done! The Black Queen [[bQ]] cannot escape now, and we say she is **pinned** (she cannot move without breaking the hive!).

After:

> Well done! The Black Queen [[bQ]] is pinned: moving her would split the hive.

### Tip 2

Before:

> Tip: An attack often starts by pinning the opponent's Queen [[bQ]]!

After:

> Tip: Pinning the opponent's Queen [[bQ]] can start an attack.

## 15. Spider

Removed repeated 'no more, no less'; made wrong-piece feedback direct.

Lesson ID: bugs.spider
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Spider [[wS1]] crawls exactly three spaces around the edge of the hive in one move. It cannot stop early or take extra steps.

After:

> The Spider [[wS1]] crawls **exactly three hexes** per move.
> All three steps happen in one turn.

### Description 2

Before:

> It cannot revisit a space during the same move, and it must keep sliding in contact with the hive.

After:

> It must stay touching the hive and cannot revisit a hex during that move.

### Instruction 1

Before:

> You play as White. Click your Spider [[wS1]] to see where it can go. It must move exactly 3 spaces around the hive, no more, no less.

After:

> You play as White. The Spider [[wS1]] must crawl exactly three hexes.
>
> Click it, then choose a destination.

### Wrong-move message 1

Before:

> Move your Spider. It must crawl exactly 3 spaces around the hive.

After:

> Move your Spider, not another piece.

### Instruction 2

Before:

> Well done! Feel free to restart the lesson to try a different move.

After:

> Well done! Restart to try the other destination.

### Tip 2

Before:

> Tip: Place your Spider [[wS1]] so it threatens to pin two opponent pieces at the same time!

After:

> Tip: A Spider [[wS1]] can threaten to pin two pieces at once.

## 16. Spiders cannot backtrack

Explained backtracking in the description and made the observation-only task explicit.

Lesson ID: bugs.spider_dead_end
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> A Spider [[wS1]] cannot revisit a space during the same move. If its only way to continue would take it back where it came from, it cannot complete its three steps.

After:

> A Spider [[wS1]] cannot revisit a hex during a move.
> Going back would prevent it from completing three steps.

### Instruction 1

Before:

> The highlighted Spider [[wS1]] is not pinned, but it cannot complete 3 steps without backtracking. A Spider [[wS1]] cannot return to a space it already visited during the same move. There is nothing to move here: continue to the next lesson.

After:

> This Spider [[wS1]] has no three-step route without backtracking.
>
> There is no move to play; choose Next lesson.

## 17. The Door

Defined door before using it; removed the repeated three-step rule from the command.

Lesson ID: bugs.spider_door
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> An open Door gives a Spider [[wS1]] different routes around the hive. Every legal move still follows exactly 3 steps without backtracking.

After:

> A **door** is an opening that gives the Spider [[wS1]] a choice of paths.
> Each move still takes exactly three steps.

### Instruction 1

Before:

> You play as White. Use the Door to move your Spider [[wS1]] into the green hex beside the Black Queen [[bQ]]. Keep moving until you reach it: each Spider [[wS1]] move must crawl exactly 3 spaces. The arrows show your current options.

After:

> You play as White. Arrows show the Spider's [[wS1]] options.
>
> Keep moving it until it reaches the green hex.

### Wrong-move message 1

Before:

> Use the Door to work toward the green hex. Each Spider move must crawl exactly 3 spaces.

After:

> Follow a three-step route toward the green hex.

### Instruction 2

Before:

> Well done! Your Spider [[wS1]] used the Door to reach the space beside the Black Queen [[bQ]].

After:

> Well done! Your Spider [[wS1]] reached the green hex beside the Black Queen [[bQ]].

## 18. Spider power

Kept the solo-turn rule and both-Spiders objective in a shorter box.

Lesson ID: bugs.spider_power
Source: apis/src/tutorial/bugs.rs

### Instruction 1

Before:

> You play as White. Use both Spiders [[wS1]] [[wS2]] to surround the Black Queen [[bQ]]. You can keep moving either Spider [[wS1]] until you win.

After:

> You play as White and keep taking turns.
>
> Use both Spiders [[wS1]] [[wS2]] to surround the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Move one of your Spiders around the hive toward the Black Queen.

After:

> Move either White Spider toward the Black Queen.

## 19. Climbing

Moved stack-inspection controls out of the action box; retained stack height and covered-piece rules.

Lesson ID: bugs.beetle
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Beetle [[wB1]] moves one hex per turn. It can crawl on the ground or climb onto another piece, then move across the top of the hive.

After:

> The Beetle [[wB1]] moves one hex per turn.
> It can crawl on the ground or climb onto other pieces.

### Description 2

Before:

> There is no height limit for a stack. Only the piece on top can move; every piece covered underneath is immobile until it is uncovered.

After:

> Stacks have no height limit.
> Only the top piece can move; covered pieces must wait until uncovered.

### Description 3

Before:

> A Beetle [[wB1]] on the ground cannot squeeze through a narrow Gate. Climbing gives it a way around.

After:

> Climbing can let a Beetle [[wB1]] go around a ground-level gate.
> To inspect a stack, right-click on desktop. On mobile, hold it, then slide your finger away.

### Instruction 1

Before:

> You play as White. Climb onto the Black Ladybug [[bL]] with your Beetle [[wB1]]. Green arrows show legal moves; the red arrow points through a Gate your Beetle [[wB1]] cannot enter from the ground because of the Freedom to Move Rule.

After:

> You play as White. The red arrow is blocked by a gate.
>
> Climb onto the Black Ladybug [[bL]] with your Beetle [[wB1]].

### Wrong-move message 1

Before:

> Climb your Beetle onto the Black Ladybug.

After:

> Move your Beetle onto the Black Ladybug.

### Instruction 2

Before:

> Black replied by placing a Beetle [[bB1]]. You can look under a stack by right-clicking it on desktop, or by holding it and then sliding your finger elsewhere on mobile. Now move your Beetle [[wB1]] onto the Black Queen [[bQ]].

After:

> Black replied by placing a Beetle [[bB1]].
>
> Cover the Black Queen [[bQ]] with your Beetle [[wB1]].

### Instruction 3

Before:

> Well done! Black replied by climbing onto your Beetle [[wB1]]. The Black Queen [[bQ]] and your Beetle [[wB1]] are both covered and cannot move. Only the Black Beetle [[bB1]] on top can move.

After:

> Black replied by covering your Beetle [[wB1]].
> Only the Black Beetle [[bB1]] on top can move.

## 20. Spawn points

Defined spawn points once; moved the detailed colour rule into the description.

Lesson ID: bugs.beetle_spawn
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> For placing new pieces, a stack takes the colour of its top piece. A White Beetle [[wB1]] on top of a Black piece makes that hex count as White.

After:

> For placement, a stack takes the top piece's colour.
> Covering a Black piece with your Beetle [[wB1]] makes that hex count as White.

### Description 2

Before:

> By covering an opponent's piece, your Beetle [[wB1]] can create new spawn points: places where pieces from your reserve can enter the game.

After:

> This can create **spawn points**: new spaces where you may place reserve pieces.

### Instruction 1

Before:

> You play as White. Move your Beetle [[wB1]] one space forward onto the Black Pillbug [[bP]] to create new spawn points.

After:

> You play as White.
>
> Cover the Black Pillbug [[bP]] with your Beetle [[wB1]] to create new placement spaces.

### Instruction 2

Before:

> Black replied by placing a piece. Your Beetle [[wB1]] makes the covered Pillbug's [[bP]] hex count as White, opening new places to enter the game. Pick any piece from your reserve and place it on one of the two green hexes shared by your Grasshopper [[wG1]] and Beetle [[wB1]].

After:

> The covered hex now counts as White.
>
> Place any reserve piece on one of the two green hexes.

### Wrong-move message 2

Before:

> Pick any piece from your reserve, then choose one of the two green spawn points.

After:

> Choose a reserve piece, then one of the two green hexes.

### Instruction 3

Before:

> Well done! Your Beetle [[wB1]] changed the colour of the stack and created new spawn points for your pieces.

After:

> Well done! Your Beetle [[wB1]] created two new placement spaces.

## 21. Beetle gates

Replaced 'win the game' with the exact final move; retained the stack-height rule.

Lesson ID: bugs.beetle_gate
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Gates exist on top of the hive too. A Beetle [[wB1]] can't squeeze between two stacks that are higher than both where it starts and where it lands.

After:

> Gates exist above the ground too.
> A Beetle [[wB1]] cannot pass between stacks higher than both its starting and landing levels.

### Instruction 1

Before:

> Your Beetle [[wB1]] on the black Queen [[bQ]] can't step down into the highlighted hex: the two black Beetles [[bB1]] beside the way down form a gate. Find a way around it.

After:

> The two Black Beetle [[bB1]] stacks block the red arrow.
>
> Climb your Beetle [[wB1]] onto either stack.

### Wrong-move message 1

Before:

> Climb your Beetle onto one of the two ringed black Beetles first.

After:

> Climb onto one of the two outlined Black Beetles.

### Instruction 2

Before:

> From up there nothing blocks the way down. Win the game.

After:

> The way down is now open.
>
> Move your Beetle [[wB1]] into the green hex to win.

### Wrong-move message 2

Before:

> Click your Beetle, then the highlighted hex.

After:

> Move your Beetle into the green hex.

## 22. Jump!

Removed the sniper metaphor and broad claims about what other pieces can reach.

Lesson ID: bugs.grasshopper
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Grasshopper [[wG1]] jumps in a straight line over one or more pieces and lands in the first empty hex behind them. It can't jump over an empty hex.

After:

> The Grasshopper [[wG1]] jumps over an unbroken line of pieces.
> It lands in the first empty hex beyond them.

### Description 2

Before:

> Because it jumps instead of sliding, the Grasshopper [[wG1]] is not affected by gates or by stacks!

After:

> It jumps in a straight line, over at least one piece.
> Gates and stacks do not block its jump.

### Instruction 1

Before:

> You play as White. Jump your Grasshopper [[wG1]] into the highlighted hole.

After:

> You play as White.
>
> Jump your Grasshopper [[wG1]] into the green hex.

### Wrong-move message 1

Before:

> The Grasshopper moves in straight lines only, over pieces, never over gaps.

After:

> Jump in a straight line over pieces, without crossing an empty hex.

### Instruction 2

Before:

> No walking piece could have got in there. Now jump your Grasshopper [[wG1]] out again, to the highlighted hex.

After:

> Now jump back out.
>
> Move your Grasshopper [[wG1]] to the new green hex.

### Wrong-move message 2

Before:

> The Grasshopper moves in straight lines only, over pieces, never over gaps.

After:

> Jump in a straight line over pieces, without crossing an empty hex.

### Closing description

Before:

> A Grasshopper is a sniper: it can fill a hex from far away that walking pieces can't get to at all.

After:

_Removed._

## 23. Place, jump and win

Kept the placement trap and next-turn objective; removed repeated win explanations.

Lesson ID: bugs.grasshopper_place
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Plan your Grasshopper's [[wG1]] jumping path before placing it. A tempting placement can leave it pinned before it gets a chance to jump.

After:

> Plan a Grasshopper's [[wG3]] jump before placing it.
> A poor placement can leave it pinned before it jumps.

### Instruction 1

Before:

> You play as White. Place your last Grasshopper [[wG3]] so it can jump into the last empty space around the Black Queen [[bQ]] on your next turn. Be careful: Black can pin it if you choose the wrong placement.

After:

> You play as White. Black may pin a poorly placed Grasshopper [[wG3]].
>
> Place it to win with your next jump.

### Wrong-move message 1

Before:

> Find the right placement for the last Grasshopper to attack the Black Queen!

After:

> Find a placement where your last Grasshopper can jump beside the Black Queen without being pinned.

### Instruction 2

Before:

> Black replied. Your Grasshopper [[wG3]] has a clear jumping path: jump into the green hex to surround the Black Queen [[bQ]] and win.

After:

> Black replied. Your Grasshopper [[wG3]] can now win.
>
> Jump into the green hex.

### Wrong-move message 2

Before:

> Jump your last Grasshopper into the final empty space beside the Black Queen.

After:

> Jump your last Grasshopper into the remaining space beside the Black Queen.

### Instruction 3

Before:

> Well done! You chose a safe placement, then jumped in to surround the Black Queen [[bQ]].

After:

> Well done! You placed safely, then jumped in to surround the Black Queen [[bQ]].

## 24. Jumping paths

Removed redundant fixed-tile wording and the duplicate 'White wins' text.

Lesson ID: bugs.grasshopper_paths
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> A Grasshopper [[wG1]] needs an unbroken line of pieces to jump over. Use the hive to build a route for both Grasshoppers [[wG1]] toward the Black Queen [[bQ]].

After:

> Each Grasshopper [[wG2]] [[wG3]] needs an unbroken line of pieces to jump over.
> Plan several jumps to reach the Black Queen [[bQ]].

### Instruction 1

Before:

> You play as White. Keep jumping with your two Grasshoppers [[wG2]] [[wG3]] until they fill the last two spaces around the Black Queen [[bQ]]. Only your Grasshoppers [[wG2]] [[wG3]] can move; the blank tiles stay fixed.

After:

> You play as White and keep taking turns.
>
> Use both Grasshoppers [[wG2]] [[wG3]] to fill the Black Queen's [[bQ]] last two spaces.

### Wrong-move message 1

Before:

> Follow the jumping paths, or try again from the start.

After:

> Follow the green arrows, or choose Try again to restart.

### Instruction 2

Before:

> Well done! Both Grasshoppers [[wG2]] [[wG3]] filled the last spaces around the Black Queen [[bQ]]. White wins!

After:

> Well done! Both Grasshoppers [[wG2]] [[wG3]] surrounded the Black Queen [[bQ]].

## 25. Ladybug

Removed vague speed claims and a repeated comparison with crawling pieces.

Lesson ID: bugs.ladybug
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Ladybug [[wL]] always moves exactly three hexes: two steps on top of the hive, then one step down into an empty hex.

After:

> The Ladybug [[wL]] moves exactly three hexes: **two on top of the hive, then one down** into an empty hex.

### Description 2

Before:

> It can't stop on top, and it can't skip the climb. It's quick and flexible over short distances and can land inside pockets walking pieces can't reach.

After:

> It cannot stop on top or skip either climbing step.

### Instruction 1

Before:

> You play as White. Move your Ladybug [[wL]] into the walled-in pocket: up, along the top, and down. The arrows show one three-step path.

After:

> You play as White. Arrows show a three-step path.
>
> Move your Ladybug [[wL]] into the hole.

### Wrong-move message 1

Before:

> Two steps on top of the hive, then one step down.

After:

> Take two steps on top of the hive, then one step down.

### Closing description

Before:

> No walking piece could ever get in there. Ladybugs are at their best in crowded positions.

After:

_Removed._

## 26. Beetle gates again

Kept the two-move pinning goal and red-marker explanation; cut repeated gate terminology.

Lesson ID: bugs.ladybug_gate
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Ladybug [[wL]] must obey Gates on top of the hive too. She cannot squeeze between stacks that are too high.

After:

> The Ladybug [[wL]] must obey gates on top of the hive.
> High stacks can block its route.

### Description 2

Before:

> The three red hexes beside the Black Queen [[bQ]] cannot be reached by the Ladybug [[wL]] in one move. The red arrow shows a blocked climb between the two Beetle [[wB1]] stacks. Find a different route to pin the Black Queen [[bQ]] in two moves.

After:

> The red arrow shows a blocked climb.
> The Ladybug [[wL]] cannot reach the three red hexes in one move.

### Instruction 1

Before:

> You play as White. Find a legal route for your Ladybug [[wL]] to pin the Black Queen [[bQ]] in two moves. The direct route is blocked by Beetle [[wB1]] Gates.

After:

> You play as White. The direct route is blocked.
>
> Pin the Black Queen [[bQ]] with your Ladybug [[wL]] in two moves.

### Wrong-move message 1

Before:

> Move your Ladybug around the Beetle Gates so she can reach the Black Queen on the next move.

After:

> Move your Ladybug around the stacks so it can pin the Black Queen next turn.

### Instruction 2

Before:

> Your turn. Move your Ladybug [[wL]] next to the Black Queen [[bQ]] so she cannot move without breaking the hive.

After:

> Your turn again.
>
> Move your Ladybug [[wL]] beside the Black Queen [[bQ]] and pin her.

### Wrong-move message 2

Before:

> Move your Ladybug next to the Black Queen and pin her.

After:

> Move your Ladybug beside the Black Queen where she cannot leave without splitting the hive.

### Instruction 3

Before:

> Well done! Your Ladybug [[wL]] went around the Beetle [[wB1]] Gates and pinned the Black Queen [[bQ]].

After:

> Well done! Your Ladybug [[wL]] went around the gate and pinned the Black Queen [[bQ]].

## 27. The Pillbug's ability

Used move for the specific destination and lift for the general ability; replaced unexplained stunned jargon with the rule.

Lesson ID: bugs.pillbug
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Pillbug [[wP]] is very special. It can move or use its unique ability:

After:

> The Pillbug [[wP]] has two options:

### Description 2

Before:

> • When it moves, it crawls one hex at a time around the edge of the hive, like the Queen [[wQ]].

After:

> • Crawl one hex, like the Queen [[wQ]].

### Description 3

Before:

> • When using its ability, it can pick up any piece next to it, yours or your opponent's, carry it over its back, and put it down in another empty hex next to it.

After:

> • Lift an adjacent piece—yours or your opponent's—over its back into another empty hex beside the Pillbug [[wP]].

### Description 4

Before:

> There are some limitations:

After:

> Its lifting ability has limits:

### Description 5

Before:

> • It can't break the hive in the process!

After:

> • The hive must stay connected.

### Description 6

Before:

> • It can't move any piece that is part of a stack of Beetles [[wB1]].

After:

> • It cannot lift any piece from a stack, even the top one.

### Description 7

Before:

> • It can't move a piece that moved on the very last turn.

After:

> • It cannot lift the piece moved on the previous turn.

### Description 8

Before:

> • A piece moved by the Pillbug [[wP]] is also stunned for the turn right after.

After:

> • A piece moved this way cannot move on the next turn.

### Description 9

Before:

> We'll go through all of these now.

After:

_Removed._

### Instruction 1

Before:

> You play as White. Move your Beetle [[wB1]] using the Pillbug [[wP]] to attack the enemy Queen [[bQ]] in one move. Select the Beetle [[wB1]], then the green hex beside the Black Queen [[bQ]].

After:

> You play as White. Use the Pillbug's [[wP]] ability.
>
> Move your Beetle [[wB1]] into the green hex beside the Black Queen [[bQ]].

### Wrong-move message 1

Before:

> Use your Pillbug to lift the Beetle into the empty hex next to the Black Queen.

After:

> Use your Pillbug to move the Beetle into the green hex beside the Black Queen.

### Instruction 2

Before:

> Well done! Your Pillbug [[wP]] carried the Beetle [[wB1]] into an attacking position beside the Black Queen [[bQ]], all in one turn.

After:

> Well done! Your Pillbug [[wP]] moved the Beetle [[wB1]] beside the Black Queen [[bQ]] in one turn.

## 28. Pillbug, save the Queen!

Kept the two-part reset clear; separated lifting another piece from moving the Pillbug itself.

Lesson ID: bugs.pillbug_save
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Your Queen [[wQ]] is almost surrounded: she has only one empty neighbouring hex.

After:

> Your Queen [[wQ]] has only one empty neighbouring hex.

### Description 2

Before:

> Your Pillbug [[wP]] can lift her out of danger. Select your Queen [[wQ]], then an empty hex beside the Pillbug [[wP]].

After:

> Your Pillbug [[wP]] can lift her out of danger.

### Description 3

Before:

> This lesson has two parts. After the rescue, the same starting position will return so you can find another way to give your Queen [[wQ]] more breathing room.

After:

> After the rescue, press Next to reset the position and try a different escape.

### Instruction 1

Before:

> Part 1 of 2. You play as White. Save your Queen [[wQ]] using your Pillbug's [[wP]] ability. Move her to any legal empty hex beside the Pillbug [[wP]].

After:

> Part 1: You play as White.
>
> Lift your Queen [[wQ]] to safety using the Pillbug [[wP]].

### Wrong-move message 1

Before:

> Select your Queen, then let the Pillbug carry her to an empty hex beside it.

After:

> Select your Queen, then an empty hex beside the Pillbug.

### Instruction 2

Before:

> Your Queen [[wQ]] is safe! Take a look at her new position, then press Next to try the second part.

After:

> Your Queen [[wQ]] is safe!
>
> Press Next to try part 2.

### Instruction 3

Before:

> Part 2 of 2. The starting position is back. Remember, your Pillbug [[wP]] can also move by itself! Move it one hex away from your Queen [[wQ]] to give her more breathing room.

After:

> Part 2: The position has reset.
>
> Move the Pillbug [[wP]] itself one hex away from your Queen [[wQ]].

### Wrong-move message 3

Before:

> The Pillbug can also move by itself! Move it one hex away from your Queen.

After:

> Move the Pillbug itself to give your Queen more space.

### Instruction 4

Before:

> Well done! You used the Pillbug [[wP]] in two ways: lifting your Queen [[wQ]] to safety, and moving the Pillbug [[wP]] itself to give her more breathing room.

After:

> Well done! You lifted your Queen [[wQ]] to safety, then created space by moving the Pillbug [[wP]] itself.

## 29. Move enemy pieces

Kept the last-moved restriction once in the description and once beside the task.

Lesson ID: bugs.pillbug_enemy
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Your Pillbug [[wP]] can lift an opponent's piece as well as one of your own.

After:

> The Pillbug [[wP]] can lift either player's pieces.

### Description 2

Before:

> There is an important limit: a piece that moved on the very last turn cannot be lifted. The highlighted Black Ant [[bA2]] has just moved, so she is protected for this turn.

After:

> It cannot lift a piece moved on the previous turn.
> The marked Black Ant [[bA2]] has just moved.

### Description 3

Before:

> The other Black Ant [[bA1]] can still be lifted. Move her to any legal empty hex beside your Pillbug [[wP]].

After:

> The other Black Ant [[bA1]] can still be lifted.

### Instruction 1

Before:

> You play as White. The highlighted Black Ant [[bA2]] has just moved, so your Pillbug [[wP]] cannot lift her. Use its ability to move the other Black Ant [[bA1]] to any legal empty hex beside it.

After:

> You play as White. The marked Ant [[bA2]] cannot be lifted this turn.
>
> Use your Pillbug [[wP]] to lift Black's other Ant [[bA1]].

### Wrong-move message 1

Before:

> Lift Black's Ant 1 with your Pillbug. Ant 2 moved on the previous turn and cannot be lifted yet.

After:

> Use your Pillbug to lift the Black Ant that did not move last turn.

### Instruction 2

Before:

> Well done! Your Pillbug [[wP]] moved an enemy Ant [[bA1]]. Remember: a piece that moved on the previous turn cannot be lifted straight away.

After:

> Well done! Your Pillbug [[wP]] lifted the Black Ant [[bA1]] that had not just moved.

## 30. Beetle gates (part 3)

Kept the hidden-Beetle trap and covering restrictions; removed the repeated gate explanation.

Lesson ID: bugs.pillbug_gate
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Gates can stop a Pillbug [[wP]] from using its ability, too. Here, the two highlighted Beetle [[wB1]] stacks block your Queen [[wQ]] from being lifted over the Pillbug's [[wP]] back.

After:

> The highlighted stacks block the Pillbug [[wP]] from lifting your Queen [[wQ]].

### Description 2

Before:

> Move a White Beetle [[wB1]] to open the Gate, then rescue your Queen [[wQ]]. Be careful which stack you uncover: a Black Beetle [[bB1]] is waiting underneath one of them!

After:

> Open the gate with a White Beetle [[wB1]].
> Be careful: one stack hides a Black Beetle [[bB1]] that could cover your Queen [[wQ]].

### Instruction 1

Before:

> You play as White. Move one of your Beetles [[wB1]] [[wB2]] to open the Gate, so your Pillbug [[wP]] can rescue your Queen [[wQ]]. Avoid covering your Queen [[wQ]] or Pillbug [[wP]].

After:

> You play as White. Keep your Queen [[wQ]] and Pillbug [[wP]] uncovered.
>
> Move a Beetle [[wB1]] [[wB2]] to open the gate.

### Wrong-move message 1

Before:

> Move the White Beetle that does not uncover a Black Beetle. Keep your Queen and Pillbug uncovered.

After:

> Move the Beetle that does not free a Black Beetle. Leave your Queen and Pillbug uncovered.

### Instruction 2

Before:

> The Gate is open! Use your Pillbug [[wP]] to lift your Queen [[wQ]] to any legal empty hex beside it.

After:

> The gate is open.
>
> Use your Pillbug [[wP]] to lift your Queen [[wQ]] to safety.

### Wrong-move message 2

Before:

> Select your Queen and let the Pillbug carry her away from the Gate.

After:

> Select your Queen, then an empty hex beside the Pillbug.

### Instruction 3

Before:

> Well done! Your Beetle [[wB1]] opened the Gate without freeing the Black Beetle [[bB1]], and your Pillbug [[wP]] rescued your Queen [[wQ]].

After:

> Well done! You opened the gate safely and rescued your Queen [[wQ]] with the Pillbug [[wP]].

## 31. Neutralise the Pillbug

Moved the defensive options into the description; kept the continuous-play objective short.

Lesson ID: bugs.pillbug_deactivate
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Black Pillbug [[bP]] can rescue its Queen [[bQ]]. A Beetle [[wB1]] on top of either piece can take away that defence.

After:

> Covering the Black Pillbug [[bP]] stops its ability.
> Covering the Black Queen [[bQ]] also prevents a rescue.

### Description 2

Before:

> Coordinate your two Beetles [[wB1]] [[wB2]] to surround the Black Queen [[bQ]]. Keep them away from the Black Beetles [[bB1]], which can cover them!

After:

> Use both Beetles [[wB1]] [[wB2]] to surround the Black Queen [[bQ]].
> Black's Beetle [[bB1]] can cover yours if they get too close.

### Description 3

Before:

> You keep playing as White. Black responds only if it can rescue its Queen [[bQ]] or cover one of your Beetles [[wB1]].

After:

> You keep playing. Black responds only if it can rescue its Queen [[bQ]] or cover your Beetle [[wB1]].

### Instruction 1

Before:

> You play as White. Use your Beetles [[wB1]] [[wB2]] to surround the Black Queen [[bQ]]. Cover the Queen [[bQ]] or the Black Pillbug [[bP]] to stop a rescue, and avoid being covered by a Black Beetle [[bB1]]. Any winning combination of your two Beetles [[wB1]] works.

After:

> You play as White and keep taking turns.
>
> Surround the Black Queen [[bQ]] with both Beetles [[wB1]] [[wB2]], preventing a rescue or counterattack.

### Wrong-move message 1

Before:

> Keep the Black Queen trapped and your Beetles safe. Black can rescue its Queen or cover an exposed Beetle.

After:

> Black found a defence. Keep its Queen trapped and your Beetles out of reach of the Black Beetle.

### Instruction 2

Before:

> Well done! Your Beetles [[wB1]] [[wB2]] stopped Black's defences and surrounded the Black Queen [[bQ]].

After:

> Well done! Your Beetles [[wB1]] [[wB2]] stopped the rescue and surrounded the Black Queen [[bQ]].

## 32. Mosquito

Removed repeated two-move explanations; corrected revealed tiles to stay visible throughout the attempt.

Lesson ID: bugs.mosquito
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> The Mosquito [[wM]] copies the movement of pieces touching it. Its options change whenever it moves beside different neighbours.

After:

> The Mosquito [[wM]] copies the movement of any piece touching it.
> Its options change as its neighbours change.
> It uses one copied movement type for the whole move.

### Description 2

Before:

> Your Mosquito [[wM]] starts beside a Queen [[wQ]], Ladybug [[wL]], Ant [[wA3]] and Grasshopper [[bG1]]. Borrow the right movements to reach the last empty space around the Black Queen [[bQ]] in two moves.

After:

> Your Mosquito [[wM]] touches a Queen [[wQ]], Ladybug [[wL]], Ant [[wA3]] and Grasshopper [[bG1]].

### Description 3

Before:

> Only your Mosquito can move. Its current neighbours stay visible so you can see which movements it can copy.

After:

> Only the Mosquito [[wM]] can move here.
> Pieces it reveals stay visible during this attempt.

### Instruction 1

Before:

> You play as White. Win in two moves using your Mosquito [[wM]]. Check its neighbours to see which movements it can borrow, then surround the Black Queen [[bQ]].

After:

> You play as White. Check your Mosquito's [[wM]] neighbours.
>
> Surround the Black Queen [[bQ]] in two moves.

### Wrong-move message 1

Before:

> Find a two-move route for your Mosquito. Its movement changes with the pieces it touches.

After:

> Find a two-move route. Your Mosquito copies the pieces it touches.

### Instruction 2

Before:

> Well done! Your Mosquito [[wM]] borrowed the movements it needed to surround the Black Queen [[bQ]] in two moves.

After:

> Well done! Your Mosquito [[wM]] used its neighbours' movements to surround the Black Queen [[bQ]].

## 33. The Mosqueetle

Kept the cover-the-Queen objective and mandatory Beetle movement on top; cut repeated placement reports.

Lesson ID: bugs.mosquito_climb
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> Your Mosquito [[wM]] can copy a Beetle [[bB1]] to climb onto the hive. Move beside the Black Beetle [[bB1]] to borrow its movement.

After:

> Beside a Beetle [[bB1]], your Mosquito [[wM]] can copy its climbing movement.

### Description 2

Before:

> When a Mosquito [[wM]] is on top of the hive, it must keep moving like a Beetle [[wB1]] until it goes down again, even if it no longer touches a Beetle [[wB1]].

After:

> Once on top of the hive, a Mosquito [[wM]] must move like a Beetle [[wB1]] until it comes down.

### Instruction 1

Before:

> Your goal is to cover the Black Queen [[bQ]]. You play as White: first move your Mosquito [[wM]] beside the Black Beetle [[bB1]] and pin it.

After:

> You play as White. The goal is to cover the Black Queen [[bQ]].
>
> First, pin the Black Beetle [[bB1]] with your Mosquito [[wM]].

### Wrong-move message 1

Before:

> Pin the Black Beetle from the side away from its Queen. If your Mosquito touches both, the Beetle can cover it.

After:

> Pin the Black Beetle from the side away from its Queen, or it can cover your Mosquito.

### Instruction 2

Before:

> Black replied by placing a new piece. Your Mosquito [[wM]] can copy the enemy Beetle [[bB1]]: climb on top of it.

After:

> Black replied with a placement.
>
> Climb onto the Black Beetle [[bB1]] with your Mosquito [[wM]].

### Wrong-move message 2

Before:

> Move your Mosquito on top of the Black Beetle.

After:

> Move your Mosquito onto the Black Beetle.

### Instruction 3

Before:

> Black replied by placing another piece. Your Mosquito [[wM]] must keep moving like a Beetle [[wB1]] while it is on top of the hive. Cover the Black Queen [[bQ]].

After:

> Your Mosquito [[wM]] still moves like a Beetle [[wB1]].
>
> Cover the Black Queen [[bQ]].

### Wrong-move message 3

Before:

> Move your Mosquito from the Beetle stack onto the Black Queen.

After:

> Move your Mosquito from the stack onto the Black Queen.

### Instruction 4

Before:

> Well done! Your Mosquito [[wM]] copied the Black Beetle's [[bB1]] movement, climbed onto the Beetle [[bB1]], and covered the Black Queen [[bQ]].

After:

> Well done! Your Mosquito [[wM]] copied the Black Beetle [[bB1]] and covered the Black Queen [[bQ]].

## 34. Copy the Pillbug's ability too!

Preserved the conditional Queen option and the One Hive Rule explanation; used lift for an unrestricted destination.

Lesson ID: bugs.mosquito_ability
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> A Mosquito [[wM]] beside a Pillbug [[bP]] can copy its special ability, even when that Pillbug [[bP]] belongs to your opponent.

After:

> A Mosquito [[wM]] can copy a neighbouring Pillbug's [[bP]] ability, even an opponent's.

### Description 2

Before:

> Move beside the Black Pillbug [[bP]] without pinning it. On your next turn, use your Mosquito [[wM]] to lift the Black Pillbug [[bP]] or the Black Queen [[bQ]] to another hex beside the Mosquito [[wM]].

After:

> The One Hive Rule still applies.
> A piece cannot be lifted if removing it would split the hive.

### Description 3

Before:

> The One Hive Rule still applies: a pinned piece cannot be lifted if removing it would split the hive.

After:

> Approach without pinning the Black Pillbug [[bP]], then lift an adjacent Black piece.

### Instruction 1

Before:

> You play as White. Move your Mosquito [[wM]] beside the Black Pillbug [[bP]], but do not pin it.

After:

> You play as White.
>
> Move your Mosquito [[wM]] beside the Black Pillbug [[bP]] without pinning it.

### Wrong-move message 1

Before:

> Move beside the Black Pillbug without making it a link that holds the hive together.

After:

> Approach from a side that leaves the Black Pillbug free to move.

### Conditional wrong-move message 1

Before:

> The Black Pillbug is pinned. The One Hive Rule prevents lifting it, because removing it would split the hive. Try approaching from another side.

After:

> The Black Pillbug is pinned. Lifting it would split the hive and break the One Hive Rule. Try another approach.

### Instruction 2

Before:

> Your Mosquito [[wM]] can now copy the Black Pillbug's [[bP]] ability. Select the Black Pillbug [[bP]] or Black Queen [[bQ]], then lift it to a legal empty hex beside your Mosquito [[wM]].

After:

> Your Mosquito [[wM]] can copy the Pillbug's [[bP]] ability.
>
> Select the Black Pillbug [[bP]] or Queen [[bQ]], then an empty hex beside your Mosquito [[wM]].

### Wrong-move message 2

Before:

> Use the Mosquito's copied ability to lift an available enemy piece, rather than moving your Mosquito itself.

After:

> Lift an adjacent Black piece using the Mosquito's copied ability; do not move the Mosquito itself.

### Conditional instruction 2

Before:

> Your Mosquito [[wM]] can copy the Black Pillbug's [[bP]] ability. Select the Black Pillbug [[bP]], then lift it to a legal empty hex beside your Mosquito [[wM]].

After:

> Your Mosquito [[wM]] can copy the Pillbug's [[bP]] ability.
>
> Select the Black Pillbug [[bP]], then an empty hex beside your Mosquito [[wM]].

### Instruction 3

Before:

> Well done! Your Mosquito [[wM]] copied the opponent's Pillbug [[bP]] ability and moved an enemy piece.

After:

> Well done! Your Mosquito [[wM]] copied the Pillbug's [[bP]] ability to lift an enemy piece.

## 35. Not another Mosquito

Kept the cannot-copy-an-elevated-Mosquito rule in the description; removed it from the repeated closing summary.

Lesson ID: bugs.mosquito_other
Source: apis/src/tutorial/bugs.rs

### Description 1

Before:

> A Mosquito [[wM]] on the ground cannot copy another Mosquito [[bM]], even when that Mosquito [[bM]] is on a stack and moves like a Beetle [[wB1]].

After:

> A Mosquito [[wM]] cannot copy another Mosquito [[bM]], even one on top of the hive.

### Description 2

Before:

> First pin the Black Beetle [[bB1]]. When Black covers it with its Mosquito, yours will need a new neighbour.

After:

> After you pin the Black Beetle [[bB1]], Black will cover it with a Mosquito [[bM]].

### Description 3

Before:

> Bring your Ant [[wA1]] alongside your Mosquito so it can borrow the Ant's movement, then move your Mosquito again.

After:

> Bring your Ant [[wA1]] beside your Mosquito [[wM]] to give it another movement to copy.

### Instruction 1

Before:

> You play as White. Use your Mosquito [[wM]] to pin the Black Beetle [[bB1]].

After:

> You play as White.
>
> Pin the Black Beetle [[bB1]] with your Mosquito [[wM]].

### Wrong-move message 1

Before:

> Move your Mosquito beside the Black Beetle so the Beetle cannot leave without breaking the hive.

After:

> Move your Mosquito beside the Black Beetle where leaving would split the hive.

### Instruction 2

Before:

> Black replied by covering the Beetle [[bB1]] with its Mosquito [[bM]]. Your Mosquito [[wM]] cannot copy it and is stuck! Move your Ant [[wA1]] beside your Mosquito [[wM]] to give it a movement it can use.

After:

> Black replied with a Mosquito [[bM]], which yours cannot copy.
>
> Move your Ant [[wA1]] beside your Mosquito [[wM]] to free it.

### Wrong-move message 2

Before:

> Move your Ant beside your Mosquito, keeping the Mosquito free to move without breaking the hive.

After:

> Move your Ant beside the Mosquito without pinning it.

### Instruction 3

Before:

> Black replied with a new piece beside its Pillbug [[bP]]. Your Mosquito [[wM]] can copy your Ant [[wA1]] now: make any legal move with the Mosquito [[wM]].

After:

> Your Mosquito [[wM]] can now copy the Ant [[wA1]].
>
> Make any legal move with the Mosquito [[wM]].

### Wrong-move message 3

Before:

> Select your Mosquito and make any legal move using its new Ant neighbour.

After:

> Move your Mosquito using its Ant neighbour.

### Instruction 4

Before:

> Well done! A Mosquito [[wM]] cannot copy another Mosquito [[bM]], even if the other Mosquito [[bM]] is on top of the hive, but your Ant [[wA1]] gave your Mosquito [[wM]] the movement it needed.

After:

> Well done! Your Ant [[wA1]] freed the Mosquito [[wM]] by giving it a movement to copy.

## 36. A bit more about hivegame.com

Deferred clock notation to lesson 37 and removed unexplained rated-game terminology; clarified the read-only tour.

Lesson ID: online.create_join
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> The homepage gives you three ways to start playing: choose a ready-made time control, create a custom game, or join an open game.

After:

> The homepage lets you create a game or join one.
> This is a read-only tour; use Continue to follow along.

### Instruction 1

Before:

> Choose one of these ready-made time controls to start a game quickly. The two numbers show minutes per player and seconds added after each move.

After:

> These buttons start games with preset clocks.
>
> Press Continue to see Custom.

### Instruction 2

Before:

> Choose Custom to set your own time control and game options, including whether the game is rated and which colour you prefer.

After:

> Custom lets you choose your game options.
>
> Press Continue to see open games.

### Instruction 3

Before:

> You can also join a game someone else has created. Check the time control and game details in the open challenges list, then choose Join.

After:

> Join accepts a game another player created.
> Check its clock before joining.

### Closing description

Before:

> Pick a comfortable time control for your first games, so you have time to think.

After:

> Tip: Start with a longer clock so you have time to think.

## 37. Reading a time control

Explained clock notation without using increment jargon; removed a repeated preview of the task.

Lesson ID: online.time_controls
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> A time control has two numbers. In 5+4, each player starts with 5 minutes and gains 4 seconds after every move.

After:

> A clock such as **5+4** gives each player 5 minutes, plus 4 seconds after each move.

### Description 2

Before:

> Here we use 1+0: one minute each, with no increment. Your clock runs only during your turn. Reaching zero loses the game.

After:

> Here, **1+0** means one minute each, with no extra seconds.
> Your clock runs only on your turn. Reaching zero loses the game.

### Description 3

Before:

> Make a move before your minute runs out, then watch Black's clock count down.

After:

_Removed._

### Instruction 1

Before:

> You play as White. Both players have one minute. Make any legal move or placement before your clock reaches zero, then watch your opponent's clock.

After:

> You play as White. You have one minute.
>
> Play any legal move or placement before your clock reaches zero.

### Wrong-move message 1

Before:

> Make a legal White move before your minute runs out.

After:

> Make a legal White move before your clock reaches zero.

### Instruction 2

Before:

> Your clock has stopped and Black's clock is running. Black will not move in this exercise: watch what happens when the clock reaches zero.

After:

> Your clock has stopped. Black will not move.
>
> Watch Black's clock count down to zero.

### Instruction 3

Before:

> Black ran out of time, so White wins! The red clock shows which player lost on time.

After:

> Black ran out of time. White wins!
> The losing clock turns red.

## 38. Undo, offer a draw and resign

Kept the opponent's choice and confirmation behaviour; removed repeated consequences.

Lesson ID: online.controls
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> These three controls appear beside the board. In a real game, the first click selects an action and a second click confirms it.

After:

> These controls sit beside the board.
> In a game, click once to select an action and again to confirm.
> These tutorial controls do not affect a real game.

### Instruction 1

Before:

> Undo requests a takeback of your last move. Your opponent can accept or decline the request; a request does not undo a move automatically.

After:

> Undo asks to take back your last move.
> Your opponent must agree.

### Instruction 2

Before:

> Offer draw asks your opponent to agree to a draw. If they accept, the game ends with no winner. If they decline, the game continues.

After:

> Offer draw asks to end the game as a draw.
> If your opponent declines, play continues.

### Instruction 3

Before:

> Resign ends the game immediately and gives your opponent the win. Use it when you decide you no longer want to continue the game.

After:

> Resign ends the game and gives your opponent the win.

## 39. Change settings

Removed three repeated directions to Config; added what the settings page actually contains.

Lesson ID: online.settings
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> Open your profile menu at the top right and choose Config to find your settings.

After:

> Open your profile menu at the top right, then choose **Config**.

### Instruction 1

Before:

> This is the actual Settings page. You can reach it by opening your profile menu at the top right and choosing Config.

After:

> Settings controls board appearance and move confirmation.
>
> Scroll to explore this read-only page.

### Closing description

Before:

> You can return to Config from your profile menu whenever you want.

After:

_Removed._

## 40. Correspondence games

Kept the two flavours as short bullets using the actual form labels; removed unrelated untimed-game text.

Lesson ID: online.correspondence
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> Real-time games are played in one sitting. Correspondence games are played over days: make your move when it suits you, and the site tells you when it is your turn.

After:

> Correspondence games run over days rather than one sitting.
> Play when it suits you; the site shows when it is your turn.

### Description 2

Before:

> Look for the envelope symbol to recognise correspondence games.

After:

> The envelope icon marks correspondence games.

### Instruction 1

Before:

> Correspondence comes in two flavours:
> - **Days per move:** you have that many days for each move. The allowance refills after every move.
> - **Total days:** each player has that many days for the whole game.
> You can also play untimed games with no clock.

After:

> Two clock options:
>
> - **Days per move:** refills after each move.
> - **Total time each:** lasts the whole game.

## 41. Tournaments

Replaced a long list of form fields with the essential purpose and an explicit Continue command.

Lesson ID: online.tournaments
Source: apis/src/tutorial/online.rs

### Description 1

Before:

> The Tournaments menu lets you browse events and organise your own.

After:

> Use **Tournaments** to browse events or organise one.

### Instruction 1

Before:

> Browse tournaments here to see their status, time control and participants. Open an event in the real site to read its rules before joining.

After:

> This page lists events and their details.
>
> Press Continue to see the creation page.

### Instruction 2

Before:

> Create Tournament lets an organiser choose a name, description, format, player limits, time control and start rules. When you organise an event, explain its rules clearly to everyone joining.

After:

> Choose the event's name, rules, format and clock here.
>
> Press Continue to finish this read-only tour.

## 42. You're ready!

Shortened the farewell and kept the feedback destination.

Lesson ID: online.ready
Source: apis/src/tutorial/online.rs

### Instruction 1

Before:

> You've learned everything you need to get started. Now comes the best part: play and enjoy! Let us know if you have any feedback using the links in the homepage footer.

After:

> You're ready—play and enjoy!
>
> Share feedback through the links in the homepage footer.

## Shared feedback and display

Source: apis/src/providers/tutorial/context.rs

> Before: Your time ran out. Press Try again to restart with one minute.

> After: Your time ran out. Choose Try again to restart.

Source: apis/src/components/organisms/tutorial/side_board.rs

> Before: Takeback requested. Your opponent sees the request and can accept, which undoes your last move, or decline it.

> After: Your opponent must agree to undo your last move.

Source: apis/src/components/organisms/tutorial/side_board.rs

> Before: Draw offered. If your opponent accepts, the game ends and nobody wins. If not, play goes on.

> After: Your opponent can accept the draw or keep playing.

Source: apis/src/components/organisms/tutorial/side_board.rs

> Before: You resigned. In a real game that ends it at once and your opponent wins. Here nothing happened, so carry on.

> After: In a real game, resigning gives your opponent the win. This tutorial is unchanged.

Source: apis/src/components/organisms/tutorial/lesson_panel.rs

> Before: Good! It's back where it started, so try the next one.

> After: Back at the start. Try the next destination.

Source: apis/src/components/organisms/tutorial/lesson_panel.rs

> Before: Look around as much as you like. Press Continue when you're ready to move.

> After: Press Continue before making a move.

## Homepage and navigation

- Tutorial invitation: **New to Hive?** followed only by the **Tutorial** button.
- Removed the explanatory paragraph and changing Start / Continue / Review button labels. The existing resume destination is preserved.
- Added **Tutorial** to the Learn section in both desktop and mobile navigation.
- English creation heading: **Create a game** → **Create new game**, matching upstream/main.
- Added **Join a game** above the Humans / Bots challenge panel, using the creation heading's styling.
- New English menu and heading keys fall back to English in locales without translations.

## Reviewed and kept

- Lesson 11: instruction 2.
- Lesson 18: instruction 2.
- Lesson 19: wrong-move message 2.
- Lesson 20: wrong-move message 1.
- Lesson 37: wrong-move message 2.

## Validation

- 92 existing lesson logic tests pass after updating three assertions that depended on the old wording.
- Frontend and server builds pass.
- Desktop and 390 x 844 mobile checks pass for the homepage, Learn menu, lesson descriptions and instruction layout.
- Sample first lesson, Pillbug lesson and online clock lesson render correctly.
- Browser cache initially loaded an older bundle; a fresh local origin verified the rebuilt app.
- git diff --check passes.
- No commit or push was made. The pre-existing generated db/src/schema.rs change was left alone.
