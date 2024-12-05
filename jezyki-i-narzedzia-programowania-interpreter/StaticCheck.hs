module StaticCheck where

import Control.Monad.Except
import Control.Monad.Reader
import Control.Monad.State
import qualified Data.Map as Map
import Latte.Abs

data Env = Env
  { variablesTypes :: Map.Map Ident Type,
    allowedReturnType :: Type,
    insideLoop :: Bool
  }

np = BNFC'NoPosition

builtinVariablesTypes :: Map.Map Ident Type
builtinVariablesTypes =
  Map.fromList
    [ (Ident "printInt", Fun np (Void np) [ArgType np (Int np)]),
      (Ident "printString", Fun np (Void np) [ArgType np (Str np)]),
      (Ident "error", Fun np (Void np) [])
    ]

-- Exceptions are for static check errors and won't be catched.
-- Order of monads doesn't matter here.
type StaticCheck = ReaderT Env (Except String)

runStaticCheck :: Env -> StaticCheck () -> Either String ()
runStaticCheck startingEnv computation = runExcept $ runReaderT computation startingEnv

initialEnv =
  Env
    { variablesTypes = builtinVariablesTypes,
      allowedReturnType = Void np,
      insideLoop = False
    }

-- unpacks statement that declares variables so their types can be added to environment
varsTypesFromDecl :: Stmt -> [(Ident, Type)]
varsTypesFromDecl (Decl _pos typ decls) = map unpack decls
  where
    unpack (Init _pos ident _expr) = (ident, typ)
varsTypesFromDecl _ = error "internal error: varsTypesFromDecl"

varArgTypeFromArg :: Arg -> (Ident, ArgType)
varArgTypeFromArg (Arg pos typ ident) = (ident, ArgType pos typ)
varArgTypeFromArg (ArgRef pos typ ident) = (ident, ArgTypeRef pos typ)

varTypeFromArg :: Arg -> (Ident, Type)
varTypeFromArg (Arg pos typ ident) = (ident, typ)
varTypeFromArg (ArgRef pos typ ident) = (ident, typ)

-- unpacks syntax of functions declaration into pair that can be addeds to environment for lookup
varTypeFromFnDef :: FnDef -> (Ident, Type)
varTypeFromFnDef (FnDef pos ret name arglist _body) = (name, Fun pos ret (map (snd . varArgTypeFromArg) arglist))

-- throw error with position information printed
throwErrorAtPos :: BNFC'Position -> String -> StaticCheck ()
throwErrorAtPos pos msg = throwError $ "(" ++ posToStr pos ++ ") " ++ msg
  where
    posToStr (Just (l, c)) = show l ++ ":" ++ show c
    posToStr Nothing = ""

varTypeLookup :: BNFC'Position -> Ident -> StaticCheck Type
varTypeLookup pos ident = do
  maybeType <- asks (Map.lookup ident . variablesTypes)
  case maybeType of
    Just typ -> return typ
    Nothing -> do
      throwErrorAtPos pos "use of undeclared identifier"
      return (Void np)

envWithNewVars :: [(Ident, Type)] -> Env -> Env
-- note: union prefers items from the left map when there are duplicates,
-- we want new types to override old types so the order of arguments to union
-- is very important here
envWithNewVars varlist env = env {variablesTypes = Map.union newtypes oldtypes}
  where
    oldtypes = variablesTypes env
    newtypes = Map.fromList varlist

checkExprType :: Expr -> Type -> StaticCheck ()
checkExprType gotexpr expected = do
  gotType <- checkExpr gotexpr
  when
    -- Eq implementation for Type has been modified in auto-generated Latte/Abs.hs.
    -- Thanks to this comparing doesn't fail when the BNFC'Positions are different.
    (gotType /= expected)
    ( throwErrorAtPos
        (hasPosition gotexpr)
        ("expression type mismatch: " ++ "expected " ++ showType expected ++ " got " ++ showType gotType)
    )
  where
    showType (Int pos) = "int" ++ posToStr pos
    showType (Str pos) = "string" ++ posToStr pos
    showType (Bool pos) = "boolean" ++ posToStr pos
    showType (Void pos) = "void" ++ posToStr pos
    showType (Fun _ ret1 arglist1) = ""
    posToStr (Just (l, c)) = " deduced from position (" ++ show l ++ ":" ++ show c ++ ")"
    posToStr Nothing = ""

checkProgram (Program pos fndefs) = do
  -- top level functions need to see each other
  -- to enable mutual recursion, that is why
  -- some trickery needs to be done around the
  -- environment before static checking can take place
  let fnvars = map varTypeFromFnDef fndefs
  oldenv <- ask
  let newEnv = envWithNewVars fnvars oldenv
  let mmain = Map.lookup (Ident "main") (variablesTypes newEnv)
  when
    (length (Map.fromList fnvars) /= length fnvars)
    (throwErrorAtPos pos "repeated top level function name")
  mapM_ (local (envWithNewVars fnvars) . checkFnDef) fndefs
  case mmain of
    Just (Fun pos retType argsTypes) -> do
      unless (null argsTypes) (throwError "main function cannot take any arguments")
      case retType of
        Void _ -> return ()
        Int _ -> return ()
        _ -> throwError "main function can only return void or int"
    _ -> throwError "missing main function"

checkFnDef fndef@(FnDef pos ret name arglist body@(Block _ stmts)) = do
  -- force all functions to end with return statement to prevent runtime errors
  -- where the function doesn't return the expected value
  when
    (ret /= Void np)
    ( case reverse stmts of
        (Ret _ _ : _) -> return ()
        _ -> throwErrorAtPos pos "function should end with return"
    )
  let argVars = map varTypeFromArg arglist
  when
    (length (Map.fromList argVars) /= length argVars)
    (throwErrorAtPos pos "repeated function argument name")
  local
    (envWithNewVars argVars)
    ( local (\e -> e {allowedReturnType = ret}) (checkBlock body)
    )
  return ()

checkBlock (Block pos stmts) = checkStmts stmts

checkStmts :: [Stmt] -> StaticCheck ()
checkStmts (stmt : rest) = case stmt of
  (Empty pos) -> checkRest
  (BStmt pos block) -> do
    checkBlock block
    checkRest
  (Decl pos typ items) -> do
    newvars <-
      mapM
        (\(Init pos ident expr) -> checkExprType expr typ >> return (ident, typ))
        items
    when
      (length (Map.fromList newvars) /= length newvars)
      (throwErrorAtPos pos "repeated variable name in multi declaration")
    checkRestWithNewVars newvars
  (Ass pos ident expr) -> do
    typ <- varTypeLookup pos ident
    checkExprType expr typ
    checkRest
  (Ret pos expr) -> do
    typ <- asks allowedReturnType
    checkExprType expr typ
    checkRest
  (VRet pos) -> do
    typ <- asks allowedReturnType
    unless (typ == Void np) (throwErrorAtPos pos "return without expression in a function returning non void")
    checkRest
  (Cond pos expr block) -> do
    checkExprType expr (Bool np)
    checkBlock block
    checkRest
  (CondElse pos expr block elseBlock) -> do
    checkExprType expr (Bool np)
    checkBlock block
    checkBlock elseBlock
    checkRest
  (While pos expr block) -> do
    checkExprType expr (Bool np)
    local (\e -> e {insideLoop = True}) (checkBlock block)
    checkRest
  (SExp pos expr) -> do
    checkExpr expr
    checkRest
  (Break pos) -> do
    ins <- asks insideLoop
    unless ins (throwErrorAtPos pos "break statement outside of any loop")
    checkRest
  (Continue pos) -> do
    ins <- asks insideLoop
    unless ins (throwErrorAtPos pos "continue statement outside of any loop")
    checkRest
  (InnerFnDef pos fnDef) -> do
    local (envWithNewVars [varTypeFromFnDef fnDef]) (checkFnDef fnDef)
    checkRestWithNewVars [varTypeFromFnDef fnDef]
  where
    checkRest = checkStmts rest
    checkRestWithNewVars varlist = local (envWithNewVars varlist) checkRest
checkStmts [] = return ()

checkExpr :: Expr -> StaticCheck Type
checkExpr expr = case expr of
  (EVar pos ident) -> varTypeLookup pos ident
  (ELitInt pos int) -> return (Int pos)
  (ELitTrue pos) -> return (Bool pos)
  (ELitFalse pos) -> return (Bool pos)
  (EString pos str) -> return (Str pos)
  (Neg pos arg) -> checkUnaryOp arg (Int np) (Int pos)
  (Not pos arg) -> checkUnaryOp arg (Bool np) (Bool np)
  (EMul pos lhs mulop rhs) -> checkBinaryOp lhs rhs (Int np) (Int pos)
  (EAdd pos lhs mulop rhs) -> checkBinaryOp lhs rhs (Int np) (Int pos)
  (ERel pos lhs mulop rhs) -> checkBinaryOp lhs rhs (Int np) (Bool pos)
  (EAnd pos lhs rhs) -> checkBinaryOp lhs rhs (Bool np) (Bool pos)
  (EOr pos lhs rhs) -> checkBinaryOp lhs rhs (Bool np) (Bool pos)
  (EApp pos ident exprs) -> do
    typ <- varTypeLookup pos ident
    case typ of
      Fun fnpos retType argsTypes -> do
        unless
          (length argsTypes == length exprs)
          (throwErrorAtPos pos "incorrect number of arguments in a function call")
        zipWithM_ checkExprArgType exprs argsTypes
        return retType
      _ -> throwErrorAtPos pos "variable called as if it was a function" >> return (Void np)
  where
    checkExprArgType :: Expr -> ArgType -> StaticCheck ()
    checkExprArgType gotexpr expected = case expected of
      ArgTypeRef pos typ -> do
        case gotexpr of
          (EVar pos ident) -> return ()
          expr -> void $ throwErrorAtPos (hasPosition expr) "expected argument by reference (variable), got expression"
        checkExprType gotexpr typ
      ArgType pos typ -> checkExprType gotexpr typ
    checkUnaryOp :: Expr -> Type -> Type -> StaticCheck Type
    checkUnaryOp arg argType retType = do
      checkExprType arg argType
      return retType
    checkBinaryOp :: Expr -> Expr -> Type -> Type -> StaticCheck Type
    checkBinaryOp lhs rhs argType retType = do
      checkExprType lhs argType
      checkExprType rhs argType
      return retType
