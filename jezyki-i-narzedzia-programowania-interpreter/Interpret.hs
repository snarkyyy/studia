module Interpret where

import Control.Monad.Except
import Control.Monad.Reader
import Control.Monad.State
import qualified Data.Map as Map
-- hide syntax elements, they are redefined as values
import Latte.Abs hiding (Bool, Fun, Int, Str, Type', Void)

np = BNFC'NoPosition

type Address = Int

type Memory = Map.Map Address Value

type Env = Map.Map Ident Address

data Bound
  = ByRef Ident
  | ByValue Ident

data Value
  = Void
  | Int Integer
  | Str String
  | Bool Bool
  -- keep body as a "precompiled" monad Exec,
  -- so builtin functions can be implemented outside the Latte language
  | Fun Env [Bound] (Exec ())

-- used with MonadExcept for easy to handle escape from nested statements
data Escape
  = LoopBreak
  | LoopContinue -- thrown by statement, catched by loops
  | Return Value -- thrown by return statement, catched by function call
  | Error String -- finishes program (never catched), used by runtime errors and `error` builtin

throwEscape = throwError

catchEscape = catchError

builtinVariables :: [(Ident, Value)]
builtinVariables =
  [ (Ident "printInt", Fun Map.empty [ByValue (Ident "intToPrint")] builtinPrintInt),
    (Ident "printString", Fun Map.empty [ByValue (Ident "stringToPrint")] builtinPrintString),
    (Ident "error", Fun Map.empty [] builtinError)
  ]
  where
    builtinPrintInt = do
      (Int val) <- getVariableValue (Ident "intToPrint")
      liftIO $ print val
      throwEscape $ Return Void -- return;
    builtinPrintString = do
      (Str val) <- getVariableValue (Ident "stringToPrint")
      liftIO $ putStrLn val
      throwEscape $ Return Void -- return;
    builtinError = do
      throwEscape $ Error "runtime error" -- no equivalent in the language

-- State monad has to be inside escape monad so side effect of computations
-- done just before an Escape is thrown will be visible for the code that catches an Escape.
type Exec = ReaderT Env (ExceptT Escape (StateT Memory IO))

runExec :: Env -> Memory -> Exec a -> IO (Either Escape a)
runExec startingEnv startingMem exec = evalStateT (runExceptT (runReaderT exec startingEnv)) startingMem

runProgram :: Program -> IO (Either Escape Value)
runProgram program = runExec Map.empty Map.empty exec
  where
    exec = do
      startingEnv <- putVariables builtinVariables
      local (const startingEnv) (execProgram program)

getVariableValue :: Ident -> Exec Value
getVariableValue ident = do
  (Just addr) <- asks (Map.lookup ident)
  (Just val) <- gets (Map.lookup addr)
  return val

putInMemory :: Value -> Exec Address
putInMemory val = do
  newAddr <- gets Map.size
  assignInMemory newAddr val
  return newAddr

assignInMemory :: Address -> Value -> Exec ()
assignInMemory addr val = do
  modify (Map.insert addr val)

assignToVariable :: Ident -> Value -> Exec ()
assignToVariable ident val = do
  (Just addr) <- asks (Map.lookup ident)
  assignInMemory addr val

getVariableAddress :: Ident -> Exec Address
getVariableAddress ident = do
  (Just addr) <- asks (Map.lookup ident)
  return addr

-- puts values in memory and returns modified environment
-- with new values assigned to identificators
putVariables :: [(Ident, Value)] -> Exec Env
putVariables varlist = do
  newvars <-
    mapM
      ( \(ident, value) -> do
          addr <- putInMemory value
          return (ident, addr)
      )
      varlist
  asks (envWithNewVars newvars)

envWithNewVars :: [(Ident, Address)] -> Env -> Env
-- note: union prefers items from the left map when there are duplicates
envWithNewVars varlist env = Map.union newvars env
  where
    newvars = Map.fromList varlist

boundFromArg :: Arg -> Bound
boundFromArg (Arg pos typ ident) = ByValue ident
boundFromArg (ArgRef pos typ ident) = ByRef ident

valFromFnDef :: FnDef -> Exec (Ident, Value)
valFromFnDef (FnDef pos ret name arglist body) = do
  env <- ask
  let boundlist = map boundFromArg arglist
  return (name, Fun env boundlist (execBlock body))

throwErrorAtPos :: BNFC'Position -> String -> Exec ()
throwErrorAtPos pos msg = throwEscape $ Error $ "(" ++ posToStr pos ++ ") " ++ msg
  where
    posToStr (Just (l, c)) = show l ++ ":" ++ show c
    posToStr Nothing = ""

execProgram :: Program -> Exec Value
execProgram (Program pos fnDefs) = do
  -- top level function definitions need to see one another,
  -- that is why this part requires a bit of manual allocation handling
  fnIdents <- mapM (fmap fst . valFromFnDef) fnDefs
  -- preallocate memory slots for the functions:
  fnAddrs <- mapM (const (putInMemory Void)) fnIdents
  newEnv <- asks (envWithNewVars (zip fnIdents fnAddrs))
  fnVals <- mapM (fmap snd . (local (const newEnv) . valFromFnDef)) fnDefs
  -- override preallocated memory slots with the correct function values:
  zipWithM_ assignInMemory fnAddrs fnVals
  local (const newEnv) (execExpr (EApp np (Ident "main") []))

execBlock (Block pos stmts) = execStmts stmts

execStmts :: [Stmt] -> Exec ()
execStmts (stmt : rest) =
  case stmt of
    (Empty pos) -> execRest
    (BStmt pos block) -> do
      execBlock block
      execRest
    (Decl pos typ items) -> do
      newvars <-
        mapM
          ( \(Init pos ident expr) -> do
              val <- execExpr expr
              return (ident, val)
          )
          items
      execRestWithNewVars newvars
    (Ass pos ident expr) -> do
      val <- execExpr expr
      assignToVariable ident val
      execRest
    (Ret pos expr) -> do
      val <- execExpr expr
      throwEscape $ Return val
    (VRet pos) -> do
      throwEscape $ Return Void
    (Cond pos expr block) -> do
      (Bool eval) <- execExpr expr
      when eval (void $ execBlock block)
      execRest
    (CondElse pos expr block elseBlock) -> do
      (Bool eval) <- execExpr expr
      when eval (void $ execBlock block)
      unless eval (void $ execBlock elseBlock)
      execRest
    w@(While pos expr block) -> do
      (Bool eval) <- execExpr expr
      if eval
        then do
          reEval <-
            catchEscape
              (execBlock block >> return True)
              ( \e -> case e of
                  LoopBreak -> return False
                  LoopContinue -> return True
                  r -> throwEscape r >> return False
              )
          if reEval
            then execStmts (w : rest)
            else execRest
        else execRest
    (SExp pos expr) -> do
      execExpr expr
      execRest
    (Break pos) -> throwEscape LoopBreak
    (Continue pos) -> throwEscape LoopContinue
    (InnerFnDef pos fnDef) -> do
      (fnIdent, _) <- valFromFnDef fnDef
      fnAddr <- putInMemory Void
      newEnv <- asks (envWithNewVars [(fnIdent, fnAddr)])
      (_, fnVal) <- local (const newEnv) (valFromFnDef fnDef)
      assignInMemory fnAddr fnVal
      local (const newEnv) (execStmts rest)
  where
    execRest = execStmts rest
    execRestWithNewVars varlist = do
      newEnv <- putVariables varlist
      local (const newEnv) execRest
execStmts [] = return ()

execExpr :: Expr -> Exec Value
execExpr expr = case expr of
  (EVar pos ident) -> getVariableValue ident
  (ELitInt pos int) -> return $ Int int
  (ELitTrue pos) -> return $ Bool True
  (ELitFalse pos) -> return $ Bool False
  (EApp pos ident exprs) -> do
    (Fun env bounds comp) <- getVariableValue ident
    argVars <-
      zipWithM
        ( \b e -> case (b, e) of
            (ByValue ident, expr) -> do
              val <- execExpr expr
              addr <- putInMemory val
              return (ident, addr)
            (ByRef ident, EVar pos refident) -> do
              addr <- getVariableAddress refident
              return (ident, addr)
        )
        bounds
        exprs
    let exeEnv = envWithNewVars argVars env
    catchEscape
      (local (const exeEnv) comp >> return Void)
      ( \e -> case e of
          Return val -> return val
          r -> throwEscape r >> return Void
      )
  (EString pos str) -> return $ Str str
  (Neg pos arg) -> do
    (Int val) <- execExpr arg
    return $ Int (-val)
  (Not pos arg) -> do
    (Bool val) <- execExpr arg
    return $ Bool (not val)
  (EMul pos lhs mulOp rhs) -> do
    (Int lhs) <- execExpr lhs
    (Int rhs) <- execExpr rhs
    val <- case mulOp of
      Times _ -> return $ lhs * rhs
      Div pos -> do
        when
          (rhs == 0)
          (throwErrorAtPos pos "tried to divide by zero")
        return $ lhs `div` rhs
      Mod pos -> do
        when
          (rhs == 0)
          (throwErrorAtPos pos "tried to calculate remainder modulo zero")
        return $ lhs `mod` rhs
    return $ Int val
  (EAdd pos lhs addOp rhs) -> do
    (Int lhs) <- execExpr lhs
    (Int rhs) <- execExpr rhs
    val <- case addOp of
      Plus _ -> return $ lhs + rhs
      Minus _ -> return $ lhs - rhs
    return $ Int val
  (ERel pos lhs relOp rhs) -> do
    (Int lhs) <- execExpr lhs
    (Int rhs) <- execExpr rhs
    val <- case relOp of
      LTH _ -> return $ lhs < rhs
      LE _ -> return $ lhs <= rhs
      GTH _ -> return $ lhs > rhs
      GE _ -> return $ lhs >= rhs
      EQU _ -> return $ lhs == rhs
      NE _ -> return $ lhs /= rhs
    return $ Bool val
  (EAnd pos lhs rhs) -> do
    (Bool lhs) <- execExpr lhs
    if lhs == False
      then return $ Bool False
      else do
        (Bool rhs) <- execExpr rhs
        return $ Bool (lhs && rhs)
  (EOr pos lhs rhs) -> do
    (Bool lhs) <- execExpr lhs
    if lhs == True
      then return $ Bool True
      else do
        (Bool rhs) <- execExpr rhs
        return $ Bool (lhs || rhs)
